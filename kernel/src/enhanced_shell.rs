// Enhanced shell with modern features
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{print, println, serial_println};
use crate::interrupts::keyboard::{KeyEvent, KeyCode};
use crate::process::{ProcessManager, ProcessId, PROCESS_MANAGER};
use crate::process::thread::{ThreadManager, ThreadId, THREAD_MANAGER};

const MAX_COMMAND_LENGTH: usize = 1024;
const HISTORY_SIZE: usize = 100;
const MAX_COMPLETIONS: usize = 10;

// Command aliases
lazy_static! {
    static ref ALIASES: Mutex<BTreeMap<String, String>> = Mutex::new({
        let mut m = BTreeMap::new();
        m.insert(String::from("ll"), String::from("ls -l"));
        m.insert(String::from("la"), String::from("ls -a"));
        m.insert(String::from(".."), String::from("cd .."));
        m.insert(String::from("~"), String::from("cd /home"));
        m
    });
    
    static ref ENV_VARS: Mutex<BTreeMap<String, String>> = Mutex::new({
        let mut m = BTreeMap::new();
        m.insert(String::from("PATH"), String::from("/bin:/usr/bin"));
        m.insert(String::from("HOME"), String::from("/home"));
        m.insert(String::from("USER"), String::from("admin"));
        m.insert(String::from("SHELL"), String::from("/bin/sh"));
        m.insert(String::from("PWD"), String::from("/"));
        m
    });
}

#[derive(Clone)]
struct CommandLine {
    buffer: String,
    cursor_pos: usize,
    display_offset: usize, // For horizontal scrolling
}

impl CommandLine {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            display_offset: 0,
        }
    }
    
    fn insert_char(&mut self, c: char) {
        if self.buffer.len() < MAX_COMMAND_LENGTH {
            self.buffer.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
            self.redraw();
        }
    }
    
    fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.buffer.remove(self.cursor_pos);
            self.redraw();
        }
    }
    
    fn delete_forward(&mut self) {
        if self.cursor_pos < self.buffer.len() {
            self.buffer.remove(self.cursor_pos);
            self.redraw();
        }
    }
    
    fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.update_cursor();
        }
    }
    
    fn move_right(&mut self) {
        if self.cursor_pos < self.buffer.len() {
            self.cursor_pos += 1;
            self.update_cursor();
        }
    }
    
    fn move_home(&mut self) {
        self.cursor_pos = 0;
        self.update_cursor();
    }
    
    fn move_end(&mut self) {
        self.cursor_pos = self.buffer.len();
        self.update_cursor();
    }
    
    fn move_word_left(&mut self) {
        // Skip spaces
        while self.cursor_pos > 0 && self.buffer.chars().nth(self.cursor_pos - 1) == Some(' ') {
            self.cursor_pos -= 1;
        }
        // Skip word
        while self.cursor_pos > 0 && self.buffer.chars().nth(self.cursor_pos - 1) != Some(' ') {
            self.cursor_pos -= 1;
        }
        self.update_cursor();
    }
    
    fn move_word_right(&mut self) {
        // Skip current word
        while self.cursor_pos < self.buffer.len() && self.buffer.chars().nth(self.cursor_pos) != Some(' ') {
            self.cursor_pos += 1;
        }
        // Skip spaces
        while self.cursor_pos < self.buffer.len() && self.buffer.chars().nth(self.cursor_pos) == Some(' ') {
            self.cursor_pos += 1;
        }
        self.update_cursor();
    }
    
    fn delete_word(&mut self) {
        let start = self.cursor_pos;
        self.move_word_left();
        self.buffer.drain(self.cursor_pos..start);
        self.redraw();
    }
    
    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = 0;
        self.display_offset = 0;
    }
    
    fn set(&mut self, text: &str) {
        self.buffer = String::from(text);
        self.cursor_pos = self.buffer.len();
        self.redraw();
    }
    
    fn redraw(&self) {
        // Clear current line
        print!("\r\x1b[K");
        self.print_prompt();
        print!("{}", self.buffer);
        self.update_cursor();
    }
    
    fn update_cursor(&self) {
        // Move cursor to correct position
        print!("\r");
        self.print_prompt();
        for _ in 0..self.cursor_pos {
            print!("\x1b[C"); // Move cursor right
        }
    }
    
    fn print_prompt(&self) {
        let cwd = ENV_VARS.lock().get("PWD").cloned().unwrap_or_else(|| String::from("/"));
        let user = ENV_VARS.lock().get("USER").cloned().unwrap_or_else(|| String::from("admin"));
        
        // Colored prompt
        print!("\x1b[32m{}@reactos\x1b[0m:\x1b[34m{}\x1b[0m$ ", user, cwd);
    }
}

pub struct EnhancedShell {
    current_line: CommandLine,
    history: Vec<String>,
    history_index: Option<usize>,
    multiline_mode: bool,
    multiline_buffer: Vec<String>,
    background_jobs: Vec<BackgroundJob>,
    job_counter: u32,
}

struct BackgroundJob {
    id: u32,
    command: String,
    status: JobStatus,
    process_id: Option<ProcessId>,
    thread_id: Option<ThreadId>,
    start_time: u64,
}

enum JobStatus {
    Running,
    Completed(i32),
    Failed(String),
}

impl EnhancedShell {
    pub fn new() -> Self {
        Self {
            current_line: CommandLine::new(),
            history: Vec::new(),
            history_index: None,
            multiline_mode: false,
            multiline_buffer: Vec::new(),
            background_jobs: Vec::new(),
            job_counter: 0,
        }
    }
    
    pub fn handle_key_event(&mut self, event: KeyEvent) {
        // Periodically check background jobs
        self.check_background_jobs();
        
        match event.code {
            KeyCode::Char(c) => {
                if event.ctrl {
                    self.handle_ctrl_key(c);
                } else {
                    self.current_line.insert_char(c as char);
                }
            }
            KeyCode::ArrowUp => self.history_up(),
            KeyCode::ArrowDown => self.history_down(),
            KeyCode::ArrowLeft => {
                if event.ctrl {
                    self.current_line.move_word_left();
                } else {
                    self.current_line.move_left();
                }
            }
            KeyCode::ArrowRight => {
                if event.ctrl {
                    self.current_line.move_word_right();
                } else {
                    self.current_line.move_right();
                }
            }
            KeyCode::Home => self.current_line.move_home(),
            KeyCode::End => self.current_line.move_end(),
            KeyCode::Delete => self.current_line.delete_forward(),
            KeyCode::Tab => self.auto_complete(),
            KeyCode::Escape => self.cancel_current(),
            _ => {}
        }
    }
    
    fn handle_ctrl_key(&mut self, key: u8) {
        match key {
            b'a' | b'A' => self.current_line.move_home(),
            b'e' | b'E' => self.current_line.move_end(),
            b'w' | b'W' => self.current_line.delete_word(),
            b'u' | b'U' => {
                // Delete from cursor to beginning
                let pos = self.current_line.cursor_pos;
                self.current_line.buffer.drain(..pos);
                self.current_line.cursor_pos = 0;
                self.current_line.redraw();
            }
            b'k' | b'K' => {
                // Delete from cursor to end
                let pos = self.current_line.cursor_pos;
                self.current_line.buffer.truncate(pos);
                self.current_line.redraw();
            }
            b'l' | b'L' => {
                // Clear screen
                crate::vga_buffer::clear_screen();
                self.print_welcome();
                self.current_line.redraw();
            }
            b'c' | b'C' => {
                // Cancel current command
                println!("^C");
                self.current_line.clear();
                self.current_line.print_prompt();
            }
            b'd' | b'D' => {
                // EOF/Exit
                if self.current_line.buffer.is_empty() {
                    println!("exit");
                    self.cmd_exit();
                }
            }
            b'r' | b'R' => {
                // Reverse search in history
                self.reverse_search();
            }
            b'z' | b'Z' => {
                // Suspend (not implemented in kernel)
                println!("^Z (suspend not available in kernel mode)");
            }
            b'\n' => {
                // Process command
                self.process_command();
            }
            _ => {}
        }
    }
    
    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        
        let new_index = match self.history_index {
            None => self.history.len() - 1,
            Some(idx) if idx > 0 => idx - 1,
            _ => return,
        };
        
        self.history_index = Some(new_index);
        self.current_line.set(&self.history[new_index]);
    }
    
    fn history_down(&mut self) {
        match self.history_index {
            None => return,
            Some(idx) => {
                if idx < self.history.len() - 1 {
                    self.history_index = Some(idx + 1);
                    self.current_line.set(&self.history[idx + 1]);
                } else {
                    self.history_index = None;
                    self.current_line.clear();
                    self.current_line.print_prompt();
                }
            }
        }
    }
    
    fn auto_complete(&mut self) {
        let input = &self.current_line.buffer;
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.is_empty() {
            // Complete commands
            self.complete_command("");
        } else if parts.len() == 1 && !input.ends_with(' ') {
            // Complete command
            self.complete_command(parts[0]);
        } else {
            // Complete file path
            let last_part = if input.ends_with(' ') {
                ""
            } else {
                parts.last().unwrap_or(&"")
            };
            self.complete_path(last_part);
        }
    }
    
    fn complete_command(&mut self, prefix: &str) {
        let commands = vec![
            "help", "clear", "cls", "echo", "ver", "version", "mem", "memory",
            "ps", "processes", "uptime", "ls", "dir", "cat", "type", "cd",
            "pwd", "mkdir", "rm", "rmdir", "cp", "mv", "touch", "chmod",
            "export", "env", "alias", "unalias", "history", "exit", "shutdown",
            "reboot", "exec", "run", "jobs", "fg", "bg", "kill"
        ];
        
        let matches: Vec<&str> = commands.iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .copied()
            .collect();
        
        self.show_completions(&matches, prefix);
    }
    
    fn complete_path(&mut self, prefix: &str) {
        // TODO: Implement file path completion
        // For now, just show a message
        println!("\nFile completion for '{}' not yet implemented", prefix);
        self.current_line.redraw();
    }
    
    fn show_completions(&mut self, matches: &[&str], prefix: &str) {
        match matches.len() {
            0 => {
                // No matches, beep or flash
                print!("\x07");
            }
            1 => {
                // Single match, complete it
                let completion = matches[0];
                let to_add = &completion[prefix.len()..];
                for c in to_add.chars() {
                    self.current_line.insert_char(c);
                }
                self.current_line.insert_char(' ');
            }
            _ => {
                // Multiple matches, show them
                println!();
                for (i, &cmd) in matches.iter().enumerate() {
                    if i > 0 && i % 4 == 0 {
                        println!();
                    }
                    print!("{:<20}", cmd);
                }
                println!();
                self.current_line.redraw();
            }
        }
    }
    
    fn reverse_search(&mut self) {
        println!("\n(reverse-i-search)`': ");
        // TODO: Implement interactive reverse search
        self.current_line.redraw();
    }
    
    fn cancel_current(&mut self) {
        self.current_line.clear();
        self.multiline_mode = false;
        self.multiline_buffer.clear();
        println!();
        self.current_line.print_prompt();
    }
    
    fn process_command(&mut self) {
        let command = self.current_line.buffer.clone();
        
        // Check for line continuation
        if command.ends_with('\\') {
            self.multiline_mode = true;
            let mut line = command.clone();
            line.pop(); // Remove backslash
            self.multiline_buffer.push(line);
            print!("\n> "); // Continuation prompt
            self.current_line.clear();
            return;
        }
        
        // Combine multiline if needed
        let full_command = if self.multiline_mode {
            self.multiline_buffer.push(command.clone());
            let combined = self.multiline_buffer.join(" ");
            self.multiline_buffer.clear();
            self.multiline_mode = false;
            combined
        } else {
            command.clone()
        };
        
        println!();
        
        // Add to history
        if !full_command.trim().is_empty() {
            self.history.push(full_command.clone());
            if self.history.len() > HISTORY_SIZE {
                self.history.remove(0);
            }
        }
        self.history_index = None;
        
        // Process the command
        self.execute_command(&full_command);
        
        // Clear and show new prompt
        self.current_line.clear();
        self.current_line.print_prompt();
    }
    
    fn execute_command(&mut self, command: &str) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }
        
        // Expand aliases
        let expanded = self.expand_aliases(command);
        
        // Check for variable assignments
        if self.try_variable_assignment(&expanded) {
            return;
        }
        
        // Parse pipeline
        let pipeline = self.parse_pipeline(&expanded);
        
        // Execute pipeline
        for (i, cmd) in pipeline.iter().enumerate() {
            let is_background = cmd.ends_with('&');
            let cmd = if is_background {
                &cmd[..cmd.len() - 1].trim()
            } else {
                cmd.trim()
            };
            
            if i < pipeline.len() - 1 {
                // Pipe output to next command
                println!("Piping not yet implemented: {}", cmd);
            } else if is_background {
                self.run_background(cmd);
            } else {
                self.run_foreground(cmd);
            }
        }
    }
    
    fn expand_aliases(&self, command: &str) -> String {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return String::from(command);
        }
        
        let aliases = ALIASES.lock();
        if let Some(expansion) = aliases.get(parts[0]) {
            let mut result = expansion.clone();
            for part in &parts[1..] {
                result.push(' ');
                result.push_str(part);
            }
            result
        } else {
            String::from(command)
        }
    }
    
    fn try_variable_assignment(&mut self, command: &str) -> bool {
        if let Some(eq_pos) = command.find('=') {
            let var_name = command[..eq_pos].trim();
            let var_value = command[eq_pos + 1..].trim();
            
            if var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                ENV_VARS.lock().insert(String::from(var_name), String::from(var_value));
                return true;
            }
        }
        false
    }
    
    fn parse_pipeline(&self, command: &str) -> Vec<String> {
        command.split('|')
            .map(|s| String::from(s.trim()))
            .collect()
    }
    
    fn run_background(&mut self, command: &str) {
        self.job_counter += 1;
        let job_id = self.job_counter;
        let cmd = String::from(command);
        
        // Create a new process for the background job
        let process_id = {
            let mut process_manager = PROCESS_MANAGER.lock();
            process_manager.create_process(
                format!("bg_job_{}", job_id),
                Some(process_manager.current_process.unwrap_or(ProcessId(0)))
            )
        };
        
        // Create a thread for the background job
        let thread_id = {
            let mut thread_manager = THREAD_MANAGER.lock();
            thread_manager.create_thread(process_id)
        };
        
        // Add thread to process
        {
            let mut process_manager = PROCESS_MANAGER.lock();
            if let Some(process) = process_manager.get_process_mut(process_id) {
                process.add_thread(thread_id);
            }
        }
        
        // Create the background job entry
        let mut job = BackgroundJob {
            id: job_id,
            command: cmd.clone(),
            status: JobStatus::Running,
            process_id: Some(process_id),
            thread_id: Some(thread_id),
            start_time: crate::interrupts::TIMER_TICKS.load(core::sync::atomic::Ordering::SeqCst),
        };
        
        println!("[{}] {} &", job.id, command);
        
        // Spawn the actual background task
        self.spawn_background_task(job_id, cmd.clone());
        
        self.background_jobs.push(job);
    }
    
    fn spawn_background_task(&mut self, job_id: u32, command: String) {
        // Clone necessary data for the background task
        let jobs_ref = &mut self.background_jobs;
        
        // Create a simple task that simulates command execution
        // In a real implementation, this would parse and execute the actual command
        crate::task::spawn(crate::task::Task::new(
            &format!("bg_job_{}", job_id),
            move || {
                // Simulate command execution
                serial_println!("Background job {} started: {}", job_id, command);
                
                // Parse and execute the command
                // For now, we'll simulate execution with a delay
                for _ in 0..10 {
                    // Simulate work
                    for _ in 0..1000000 {
                        core::hint::spin_loop();
                    }
                }
                
                serial_println!("Background job {} completed: {}", job_id, command);
            }
        ));
    }
    
    fn update_job_status(&mut self, job_id: u32, new_status: JobStatus) {
        for job in &mut self.background_jobs {
            if job.id == job_id {
                job.status = new_status;
                break;
            }
        }
    }
    
    fn check_background_jobs(&mut self) {
        // Check the status of background jobs and update their status
        let current_time = crate::interrupts::TIMER_TICKS.load(core::sync::atomic::Ordering::SeqCst);
        
        for job in &mut self.background_jobs {
            if let JobStatus::Running = job.status {
                // Check if the process/thread is still running
                if let Some(process_id) = job.process_id {
                    let process_manager = PROCESS_MANAGER.lock();
                    if let Some(process) = process_manager.get_process(process_id) {
                        match process.state {
                            crate::process::ProcessState::Terminated => {
                                job.status = JobStatus::Completed(0);
                                println!("\n[{}]+ Done                    {}", job.id, job.command);
                            }
                            _ => {
                                // Still running
                            }
                        }
                    } else {
                        // Process not found, mark as failed
                        job.status = JobStatus::Failed(String::from("Process terminated unexpectedly"));
                        println!("\n[{}]- Failed                  {}", job.id, job.command);
                    }
                }
            }
        }
        
        // Clean up completed jobs that have been notified
        self.background_jobs.retain(|job| {
            !matches!(job.status, JobStatus::Completed(_)) 
        });
    }
    
    fn run_foreground(&mut self, command: &str) {
        // Parse redirections
        let (cmd, input_file, output_file, append) = self.parse_redirections(command);
        
        // Split command and arguments
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        
        // Execute built-in or external command
        match parts[0] {
            "help" => self.cmd_help(),
            "clear" | "cls" => self.cmd_clear(),
            "echo" => self.cmd_echo(&parts[1..]),
            "export" => self.cmd_export(&parts[1..]),
            "env" => self.cmd_env(),
            "alias" => self.cmd_alias(&parts[1..]),
            "unalias" => self.cmd_unalias(&parts[1..]),
            "history" => self.cmd_history(),
            "cd" => self.cmd_cd(&parts[1..]),
            "pwd" => self.cmd_pwd(),
            "jobs" => self.cmd_jobs(),
            "exit" => self.cmd_exit(),
            _ => {
                // Try to execute as external command
                println!("Command not found: {}", parts[0]);
            }
        }
    }
    
    fn parse_redirections(&self, command: &str) -> (&str, Option<&str>, Option<&str>, bool) {
        // Simple redirection parsing
        // TODO: Implement proper parsing
        (command, None, None, false)
    }
    
    fn print_welcome(&self) {
        println!("\n\x1b[36m=====================================\x1b[0m");
        println!("\x1b[33m     ReactOS Enhanced Shell v2.0     \x1b[0m");
        println!("\x1b[36m=====================================\x1b[0m");
        println!("Type '\x1b[32mhelp\x1b[0m' for available commands");
        println!("Use \x1b[32mTab\x1b[0m for completion, \x1b[32m↑/↓\x1b[0m for history\n");
    }
    
    fn cmd_help(&self) {
        println!("\x1b[1;36mAvailable Commands:\x1b[0m");
        println!("  \x1b[32mhelp\x1b[0m          - Show this help message");
        println!("  \x1b[32mclear/cls\x1b[0m     - Clear the screen");
        println!("  \x1b[32mecho\x1b[0m [text]   - Print text to screen");
        println!("  \x1b[32mcd\x1b[0m [dir]      - Change directory");
        println!("  \x1b[32mpwd\x1b[0m           - Print working directory");
        println!("  \x1b[32mls/dir\x1b[0m [path] - List directory contents");
        println!("  \x1b[32mcat/type\x1b[0m file - Display file contents");
        println!("  \x1b[32mexport\x1b[0m VAR=val- Set environment variable");
        println!("  \x1b[32menv\x1b[0m           - Show environment variables");
        println!("  \x1b[32malias\x1b[0m         - Manage command aliases");
        println!("  \x1b[32mhistory\x1b[0m       - Show command history");
        println!("  \x1b[32mjobs\x1b[0m          - List background jobs");
        println!("  \x1b[32mexit\x1b[0m          - Exit the shell");
        println!("\n\x1b[1;36mKeyboard Shortcuts:\x1b[0m");
        println!("  \x1b[33mCtrl+A\x1b[0m        - Move to beginning of line");
        println!("  \x1b[33mCtrl+E\x1b[0m        - Move to end of line");
        println!("  \x1b[33mCtrl+W\x1b[0m        - Delete word backward");
        println!("  \x1b[33mCtrl+U\x1b[0m        - Delete to beginning of line");
        println!("  \x1b[33mCtrl+K\x1b[0m        - Delete to end of line");
        println!("  \x1b[33mCtrl+L\x1b[0m        - Clear screen");
        println!("  \x1b[33mCtrl+C\x1b[0m        - Cancel current command");
        println!("  \x1b[33mCtrl+R\x1b[0m        - Reverse search history");
        println!("  \x1b[33mTab\x1b[0m           - Auto-complete");
        println!("  \x1b[33m↑/↓\x1b[0m          - Navigate history");
    }
    
    fn cmd_clear(&self) {
        crate::vga_buffer::clear_screen();
        self.print_welcome();
    }
    
    fn cmd_echo(&self, args: &[&str]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            // Expand variables
            if arg.starts_with('$') {
                let var_name = &arg[1..];
                if let Some(value) = ENV_VARS.lock().get(var_name) {
                    print!("{}", value);
                } else {
                    print!("{}", arg);
                }
            } else {
                print!("{}", arg);
            }
        }
        println!();
    }
    
    fn cmd_export(&self, args: &[&str]) {
        if args.is_empty() {
            self.cmd_env();
            return;
        }
        
        for arg in args {
            if let Some(eq_pos) = arg.find('=') {
                let var_name = &arg[..eq_pos];
                let var_value = &arg[eq_pos + 1..];
                ENV_VARS.lock().insert(String::from(var_name), String::from(var_value));
            } else {
                println!("export: invalid format. Use: export VAR=value");
            }
        }
    }
    
    fn cmd_env(&self) {
        let vars = ENV_VARS.lock();
        for (key, value) in vars.iter() {
            println!("{}={}", key, value);
        }
    }
    
    fn cmd_alias(&self, args: &[&str]) {
        if args.is_empty() {
            let aliases = ALIASES.lock();
            for (alias, expansion) in aliases.iter() {
                println!("alias {}='{}'", alias, expansion);
            }
            return;
        }
        
        for arg in args {
            if let Some(eq_pos) = arg.find('=') {
                let alias_name = &arg[..eq_pos];
                let alias_value = &arg[eq_pos + 1..].trim_matches('\'').trim_matches('"');
                ALIASES.lock().insert(String::from(alias_name), String::from(alias_value));
            } else {
                println!("alias: invalid format. Use: alias name='command'");
            }
        }
    }
    
    fn cmd_unalias(&self, args: &[&str]) {
        for arg in args {
            ALIASES.lock().remove(*arg);
        }
    }
    
    fn cmd_history(&self) {
        for (i, cmd) in self.history.iter().enumerate() {
            println!("{:4} {}", i + 1, cmd);
        }
    }
    
    fn cmd_cd(&self, args: &[&str]) {
        let path = if args.is_empty() {
            ENV_VARS.lock().get("HOME").cloned().unwrap_or_else(|| String::from("/"))
        } else {
            String::from(args[0])
        };
        
        // Update PWD
        ENV_VARS.lock().insert(String::from("PWD"), path);
    }
    
    fn cmd_pwd(&self) {
        let pwd = ENV_VARS.lock().get("PWD").cloned().unwrap_or_else(|| String::from("/"));
        println!("{}", pwd);
    }
    
    fn cmd_jobs(&self) {
        if self.background_jobs.is_empty() {
            println!("No background jobs");
            return;
        }
        
        for job in &self.background_jobs {
            let status = match &job.status {
                JobStatus::Running => "Running",
                JobStatus::Completed(code) => &format!("Done ({})", code),
                JobStatus::Failed(err) => &format!("Failed: {}", err),
            };
            println!("[{}] {} {}", job.id, status, job.command);
        }
    }
    
    fn cmd_exit(&self) {
        println!("Goodbye!");
        // In a real implementation, would exit the shell process
        // For now, just halt
        loop {
            x86_64::instructions::hlt();
        }
    }
}

// Global shell instance
lazy_static! {
    pub static ref ENHANCED_SHELL: Mutex<Option<EnhancedShell>> = Mutex::new(None);
}

pub fn init() {
    let mut shell = EnhancedShell::new();
    shell.print_welcome();
    shell.current_line.print_prompt();
    *ENHANCED_SHELL.lock() = Some(shell);
    serial_println!("Enhanced shell initialized with modern features");
}

pub fn handle_key_event(event: KeyEvent) {
    if let Some(ref mut shell) = *ENHANCED_SHELL.lock() {
        shell.handle_key_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    
    #[test]
    fn test_background_job_creation() {
        let mut shell = EnhancedShell::new();
        
        // Run a background job
        shell.run_background("test_command");
        
        // Verify job was created
        assert_eq!(shell.background_jobs.len(), 1);
        assert_eq!(shell.background_jobs[0].id, 1);
        assert_eq!(shell.background_jobs[0].command, "test_command");
        assert!(matches!(shell.background_jobs[0].status, JobStatus::Running));
        assert!(shell.background_jobs[0].process_id.is_some());
        assert!(shell.background_jobs[0].thread_id.is_some());
    }
    
    #[test]
    fn test_multiple_background_jobs() {
        let mut shell = EnhancedShell::new();
        
        // Run multiple background jobs
        shell.run_background("job1");
        shell.run_background("job2");
        shell.run_background("job3");
        
        // Verify all jobs were created
        assert_eq!(shell.background_jobs.len(), 3);
        assert_eq!(shell.background_jobs[0].id, 1);
        assert_eq!(shell.background_jobs[1].id, 2);
        assert_eq!(shell.background_jobs[2].id, 3);
        
        // Verify each job has unique process and thread IDs
        let pid1 = shell.background_jobs[0].process_id.unwrap();
        let pid2 = shell.background_jobs[1].process_id.unwrap();
        let pid3 = shell.background_jobs[2].process_id.unwrap();
        assert_ne!(pid1, pid2);
        assert_ne!(pid2, pid3);
        assert_ne!(pid1, pid3);
    }
    
    #[test]
    fn test_job_status_update() {
        let mut shell = EnhancedShell::new();
        
        // Run a background job
        shell.run_background("test_job");
        let job_id = shell.background_jobs[0].id;
        
        // Update job status to completed
        shell.update_job_status(job_id, JobStatus::Completed(0));
        assert!(matches!(shell.background_jobs[0].status, JobStatus::Completed(0)));
        
        // Update job status to failed
        shell.update_job_status(job_id, JobStatus::Failed(String::from("Test error")));
        assert!(matches!(shell.background_jobs[0].status, JobStatus::Failed(_)));
    }
    
    #[test]
    fn test_job_cleanup() {
        let mut shell = EnhancedShell::new();
        
        // Run multiple background jobs
        shell.run_background("job1");
        shell.run_background("job2");
        shell.run_background("job3");
        
        // Mark some jobs as completed
        shell.update_job_status(1, JobStatus::Completed(0));
        shell.update_job_status(3, JobStatus::Completed(0));
        
        // Check background jobs (which should clean up completed jobs)
        shell.check_background_jobs();
        
        // Only job2 should remain (still running)
        assert_eq!(shell.background_jobs.len(), 1);
        assert_eq!(shell.background_jobs[0].id, 2);
    }
    
    #[test]
    fn test_jobs_command_output() {
        let mut shell = EnhancedShell::new();
        
        // Run multiple background jobs with different statuses
        shell.run_background("running_job");
        shell.run_background("completed_job");
        shell.run_background("failed_job");
        
        // Update statuses
        shell.update_job_status(2, JobStatus::Completed(0));
        shell.update_job_status(3, JobStatus::Failed(String::from("Test failure")));
        
        // Call cmd_jobs to verify it doesn't panic
        shell.cmd_jobs();
        
        // Verify jobs are still tracked correctly
        assert_eq!(shell.background_jobs.len(), 3);
    }
    
    #[test]
    fn test_concurrent_job_execution() {
        let mut shell = EnhancedShell::new();
        
        // Launch multiple jobs concurrently
        for i in 0..5 {
            shell.run_background(&format!("concurrent_job_{}", i));
        }
        
        // Verify all jobs were created
        assert_eq!(shell.background_jobs.len(), 5);
        
        // Verify all jobs are running
        for job in &shell.background_jobs {
            assert!(matches!(job.status, JobStatus::Running));
        }
        
        // Verify job IDs are sequential
        for (i, job) in shell.background_jobs.iter().enumerate() {
            assert_eq!(job.id, (i + 1) as u32);
        }
    }
    
    #[test]
    fn test_background_job_with_process_termination() {
        let mut shell = EnhancedShell::new();
        
        // Run a background job
        shell.run_background("terminating_job");
        
        // Get the process ID
        let process_id = shell.background_jobs[0].process_id.unwrap();
        
        // Terminate the process
        {
            let mut process_manager = PROCESS_MANAGER.lock();
            process_manager.terminate_process(process_id);
        }
        
        // Check background jobs to update status
        shell.check_background_jobs();
        
        // Job should be marked as completed
        assert!(matches!(shell.background_jobs[0].status, JobStatus::Completed(_)));
    }
}
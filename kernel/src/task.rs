use alloc::string::String;
use alloc::boxed::Box;

pub struct Task {
    pub name: String,
    pub function: Box<dyn Fn() + Send + 'static>,
}

impl Task {
    pub fn new<F>(name: &str, function: F) -> Self 
    where
        F: Fn() + Send + 'static
    {
        Self {
            name: String::from(name),
            function: Box::new(function),
        }
    }
    
    pub fn run(&self) {
        (self.function)();
    }
}

pub fn spawn(task: Task) {
    // For now, just run the task immediately
    // In a real implementation, this would add the task to a scheduler
    task.run();
}
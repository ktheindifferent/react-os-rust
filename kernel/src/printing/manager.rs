use alloc::{string::String, vec::Vec, collections::BTreeMap};
use spin::RwLock;

pub struct PrinterManager {
    printer_configs: RwLock<BTreeMap<u32, PrinterConfig>>,
    printer_groups: RwLock<Vec<PrinterGroup>>,
    access_control: RwLock<AccessControlList>,
}

#[derive(Debug, Clone)]
pub struct PrinterConfig {
    pub printer_id: u32,
    pub name: String,
    pub driver: String,
    pub uri: String,
    pub location: String,
    pub description: String,
    pub ppd_file: Option<String>,
    pub options: BTreeMap<String, String>,
    pub is_shared: bool,
    pub is_default: bool,
    pub accept_jobs: bool,
    pub state_message: String,
}

#[derive(Debug, Clone)]
pub struct PrinterGroup {
    pub name: String,
    pub description: String,
    pub printer_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct AccessControlList {
    pub default_policy: AccessPolicy,
    pub user_policies: BTreeMap<String, AccessPolicy>,
    pub group_policies: BTreeMap<String, AccessPolicy>,
    pub printer_policies: BTreeMap<u32, AccessPolicy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPolicy {
    Allow,
    Deny,
    Restricted(AccessRestrictions),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessRestrictions {
    pub can_print: bool,
    pub can_manage: bool,
    pub can_view_jobs: bool,
    pub can_cancel_jobs: bool,
    pub page_limit: Option<u32>,
    pub quota_bytes: Option<u64>,
}

impl PrinterManager {
    pub fn new() -> Self {
        Self {
            printer_configs: RwLock::new(BTreeMap::new()),
            printer_groups: RwLock::new(Vec::new()),
            access_control: RwLock::new(AccessControlList::default()),
        }
    }

    pub fn add_printer_config(&self, config: PrinterConfig) {
        self.printer_configs.write().insert(config.printer_id, config);
    }

    pub fn remove_printer_config(&self, printer_id: u32) {
        self.printer_configs.write().remove(&printer_id);
    }

    pub fn get_printer_config(&self, printer_id: u32) -> Option<PrinterConfig> {
        self.printer_configs.read().get(&printer_id).cloned()
    }

    pub fn update_printer_config(&self, printer_id: u32, config: PrinterConfig) {
        self.printer_configs.write().insert(printer_id, config);
    }

    pub fn list_printer_configs(&self) -> Vec<PrinterConfig> {
        self.printer_configs.read().values().cloned().collect()
    }

    pub fn create_printer_group(&self, name: String, description: String) -> PrinterGroup {
        let group = PrinterGroup {
            name: name.clone(),
            description,
            printer_ids: Vec::new(),
        };
        self.printer_groups.write().push(group.clone());
        group
    }

    pub fn add_printer_to_group(&self, group_name: &str, printer_id: u32) {
        let mut groups = self.printer_groups.write();
        if let Some(group) = groups.iter_mut().find(|g| g.name == group_name) {
            if !group.printer_ids.contains(&printer_id) {
                group.printer_ids.push(printer_id);
            }
        }
    }

    pub fn remove_printer_from_group(&self, group_name: &str, printer_id: u32) {
        let mut groups = self.printer_groups.write();
        if let Some(group) = groups.iter_mut().find(|g| g.name == group_name) {
            group.printer_ids.retain(|&id| id != printer_id);
        }
    }

    pub fn get_printer_group(&self, name: &str) -> Option<PrinterGroup> {
        self.printer_groups.read().iter().find(|g| g.name == name).cloned()
    }

    pub fn list_printer_groups(&self) -> Vec<PrinterGroup> {
        self.printer_groups.read().clone()
    }

    pub fn set_access_policy(&self, user: &str, policy: AccessPolicy) {
        self.access_control.write().user_policies.insert(user.to_string(), policy);
    }

    pub fn check_access(&self, user: &str, printer_id: u32, action: AccessAction) -> bool {
        let acl = self.access_control.read();
        
        if let Some(printer_policy) = acl.printer_policies.get(&printer_id) {
            if !self.check_policy_allows(printer_policy, action) {
                return false;
            }
        }
        
        if let Some(user_policy) = acl.user_policies.get(user) {
            return self.check_policy_allows(user_policy, action);
        }
        
        self.check_policy_allows(&acl.default_policy, action)
    }

    fn check_policy_allows(&self, policy: &AccessPolicy, action: AccessAction) -> bool {
        match policy {
            AccessPolicy::Allow => true,
            AccessPolicy::Deny => false,
            AccessPolicy::Restricted(restrictions) => {
                match action {
                    AccessAction::Print => restrictions.can_print,
                    AccessAction::Manage => restrictions.can_manage,
                    AccessAction::ViewJobs => restrictions.can_view_jobs,
                    AccessAction::CancelJobs => restrictions.can_cancel_jobs,
                }
            }
        }
    }

    pub fn export_config(&self) -> String {
        let mut config = String::new();
        
        config.push_str("# Printer Configuration\n\n");
        
        for (id, printer) in self.printer_configs.read().iter() {
            config.push_str(&format!("[Printer_{}]\n", id));
            config.push_str(&format!("Name = {}\n", printer.name));
            config.push_str(&format!("Driver = {}\n", printer.driver));
            config.push_str(&format!("URI = {}\n", printer.uri));
            config.push_str(&format!("Location = {}\n", printer.location));
            config.push_str(&format!("Description = {}\n", printer.description));
            config.push_str(&format!("Shared = {}\n", printer.is_shared));
            config.push_str(&format!("Default = {}\n", printer.is_default));
            config.push_str(&format!("AcceptJobs = {}\n", printer.accept_jobs));
            
            if let Some(ppd) = &printer.ppd_file {
                config.push_str(&format!("PPD = {}\n", ppd));
            }
            
            for (key, value) in &printer.options {
                config.push_str(&format!("Option.{} = {}\n", key, value));
            }
            
            config.push_str("\n");
        }
        
        config
    }

    pub fn import_config(&self, config: &str) -> Result<(), &'static str> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AccessAction {
    Print,
    Manage,
    ViewJobs,
    CancelJobs,
}

impl Default for AccessControlList {
    fn default() -> Self {
        Self {
            default_policy: AccessPolicy::Allow,
            user_policies: BTreeMap::new(),
            group_policies: BTreeMap::new(),
            printer_policies: BTreeMap::new(),
        }
    }
}
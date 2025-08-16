use super::{NtStatus, object::{Handle, ObjectHeader, ObjectTrait, ObjectType}};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{vec, format};
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, Ordering};

// Windows Security Identifier (SID) structure
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sid {
    pub revision: u8,
    pub sub_authority_count: u8,
    pub identifier_authority: [u8; 6],
    pub sub_authorities: Vec<u32>,
}

impl Sid {
    pub fn new(revision: u8, identifier_authority: [u8; 6], sub_authorities: Vec<u32>) -> Self {
        Self {
            revision,
            sub_authority_count: sub_authorities.len() as u8,
            identifier_authority,
            sub_authorities,
        }
    }
    
    pub fn to_string(&self) -> String {
        let mut result = format!("S-{}-", self.revision);
        
        // Convert identifier authority
        let mut authority_value = 0u64;
        for i in 0..6 {
            authority_value = (authority_value << 8) | self.identifier_authority[i] as u64;
        }
        result.push_str(&authority_value.to_string());
        
        // Add sub-authorities
        for sub_auth in &self.sub_authorities {
            result.push_str(&format!("-{}", sub_auth));
        }
        
        result
    }
    
    pub fn from_string(sid_string: &str) -> Result<Self, NtStatus> {
        if !sid_string.starts_with("S-") {
            return Err(NtStatus::InvalidSid);
        }
        
        let parts: Vec<&str> = sid_string[2..].split('-').collect();
        if parts.len() < 3 {
            return Err(NtStatus::InvalidSid);
        }
        
        let revision = parts[0].parse::<u8>().map_err(|_| NtStatus::InvalidSid)?;
        let authority = parts[1].parse::<u64>().map_err(|_| NtStatus::InvalidSid)?;
        
        let mut identifier_authority = [0u8; 6];
        for i in 0..6 {
            identifier_authority[5 - i] = ((authority >> (i * 8)) & 0xFF) as u8;
        }
        
        let mut sub_authorities = Vec::new();
        for i in 2..parts.len() {
            sub_authorities.push(parts[i].parse::<u32>().map_err(|_| NtStatus::InvalidSid)?);
        }
        
        Ok(Self::new(revision, identifier_authority, sub_authorities))
    }
}

// Well-known SIDs
pub struct WellKnownSids;

impl WellKnownSids {
    pub fn null_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 0], vec![0])
    }
    
    pub fn world_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 1], vec![0])
    }
    
    pub fn local_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 2], vec![0])
    }
    
    pub fn creator_owner_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 3], vec![0])
    }
    
    pub fn creator_group_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 3], vec![1])
    }
    
    pub fn nt_authority_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![])
    }
    
    pub fn system_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![18])
    }
    
    pub fn local_service_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![19])
    }
    
    pub fn network_service_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![20])
    }
    
    pub fn administrators_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![32, 544])
    }
    
    pub fn users_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![32, 545])
    }
    
    pub fn guests_sid() -> Sid {
        Sid::new(1, [0, 0, 0, 0, 0, 5], vec![32, 546])
    }
}

// Access Control Entry (ACE) types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AceType {
    AccessAllowed = 0,
    AccessDenied = 1,
    SystemAudit = 2,
    SystemAlarm = 3,
    AccessAllowedCompound = 4,
    AccessAllowedObject = 5,
    AccessDeniedObject = 6,
    SystemAuditObject = 7,
    SystemAlarmObject = 8,
    AccessAllowedCallback = 9,
    AccessDeniedCallback = 10,
    AccessAllowedCallbackObject = 11,
    AccessDeniedCallbackObject = 12,
    SystemAuditCallback = 13,
    SystemAlarmCallback = 14,
    SystemAuditCallbackObject = 15,
    SystemAlarmCallbackObject = 16,
    SystemMandatoryLabel = 17,
    SystemResourceAttribute = 18,
    SystemScopedPolicyId = 19,
}

// ACE flags
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct AceFlags: u8 {
        const OBJECT_INHERIT_ACE = 0x01;
        const CONTAINER_INHERIT_ACE = 0x02;
        const NO_PROPAGATE_INHERIT_ACE = 0x04;
        const INHERIT_ONLY_ACE = 0x08;
        const INHERITED_ACE = 0x10;
        const SUCCESSFUL_ACCESS_ACE_FLAG = 0x40;
        const FAILED_ACCESS_ACE_FLAG = 0x80;
    }
}

// Access Control Entry (ACE)
#[derive(Debug, Clone)]
pub struct Ace {
    pub ace_type: AceType,
    pub ace_flags: AceFlags,
    pub access_mask: u32,
    pub sid: Sid,
}

impl Ace {
    pub fn new(ace_type: AceType, ace_flags: AceFlags, access_mask: u32, sid: Sid) -> Self {
        Self {
            ace_type,
            ace_flags,
            access_mask,
            sid,
        }
    }
    
    pub fn allow(access_mask: u32, sid: Sid) -> Self {
        Self::new(AceType::AccessAllowed, AceFlags::empty(), access_mask, sid)
    }
    
    pub fn deny(access_mask: u32, sid: Sid) -> Self {
        Self::new(AceType::AccessDenied, AceFlags::empty(), access_mask, sid)
    }
    
    pub fn audit(access_mask: u32, sid: Sid, flags: AceFlags) -> Self {
        Self::new(AceType::SystemAudit, flags, access_mask, sid)
    }
}

// Access Control List (ACL)
#[derive(Debug, Clone)]
pub struct Acl {
    pub revision: u8,
    pub aces: Vec<Ace>,
}

impl Acl {
    pub fn new() -> Self {
        Self {
            revision: 2, // ACL_REVISION
            aces: Vec::new(),
        }
    }
    
    pub fn add_ace(&mut self, ace: Ace) {
        self.aces.push(ace);
    }
    
    pub fn remove_ace(&mut self, index: usize) -> Option<Ace> {
        if index < self.aces.len() {
            Some(self.aces.remove(index))
        } else {
            None
        }
    }
    
    pub fn check_access(&self, sid: &Sid, desired_access: u32) -> bool {
        let mut allowed = 0u32;
        let mut denied = 0u32;
        
        // Process ACEs in order (denies first, then allows)
        for ace in &self.aces {
            if ace.sid == *sid {
                match ace.ace_type {
                    AceType::AccessDenied => {
                        denied |= ace.access_mask;
                    }
                    AceType::AccessAllowed => {
                        allowed |= ace.access_mask;
                    }
                    _ => {}
                }
            }
        }
        
        // Check if any denied permissions overlap with desired
        if (denied & desired_access) != 0 {
            return false;
        }
        
        // Check if all desired permissions are allowed
        (allowed & desired_access) == desired_access
    }
}

// Security Descriptor
#[derive(Debug, Clone)]
pub struct SecurityDescriptor {
    pub revision: u8,
    pub control: SecurityDescriptorControl,
    pub owner_sid: Option<Sid>,
    pub group_sid: Option<Sid>,
    pub dacl: Option<Acl>, // Discretionary ACL
    pub sacl: Option<Acl>, // System ACL
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct SecurityDescriptorControl: u16 {
        const SE_OWNER_DEFAULTED = 0x0001;
        const SE_GROUP_DEFAULTED = 0x0002;
        const SE_DACL_PRESENT = 0x0004;
        const SE_DACL_DEFAULTED = 0x0008;
        const SE_SACL_PRESENT = 0x0010;
        const SE_SACL_DEFAULTED = 0x0020;
        const SE_DACL_AUTO_INHERIT_REQ = 0x0100;
        const SE_SACL_AUTO_INHERIT_REQ = 0x0200;
        const SE_DACL_AUTO_INHERITED = 0x0400;
        const SE_SACL_AUTO_INHERITED = 0x0800;
        const SE_DACL_PROTECTED = 0x1000;
        const SE_SACL_PROTECTED = 0x2000;
        const SE_RM_CONTROL_VALID = 0x4000;
        const SE_SELF_RELATIVE = 0x8000;
    }
}

impl SecurityDescriptor {
    pub fn new() -> Self {
        Self {
            revision: 1, // SECURITY_DESCRIPTOR_REVISION
            control: SecurityDescriptorControl::empty(),
            owner_sid: None,
            group_sid: None,
            dacl: None,
            sacl: None,
        }
    }
    
    pub fn set_owner(&mut self, sid: Sid) {
        self.owner_sid = Some(sid);
        self.control.remove(SecurityDescriptorControl::SE_OWNER_DEFAULTED);
    }
    
    pub fn set_group(&mut self, sid: Sid) {
        self.group_sid = Some(sid);
        self.control.remove(SecurityDescriptorControl::SE_GROUP_DEFAULTED);
    }
    
    pub fn set_dacl(&mut self, dacl: Acl) {
        self.dacl = Some(dacl);
        self.control.insert(SecurityDescriptorControl::SE_DACL_PRESENT);
        self.control.remove(SecurityDescriptorControl::SE_DACL_DEFAULTED);
    }
    
    pub fn set_sacl(&mut self, sacl: Acl) {
        self.sacl = Some(sacl);
        self.control.insert(SecurityDescriptorControl::SE_SACL_PRESENT);
        self.control.remove(SecurityDescriptorControl::SE_SACL_DEFAULTED);
    }
}

// Privilege definitions
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    CreateToken = 2,
    AssignPrimaryToken = 3,
    LockMemory = 4,
    IncreaseQuota = 5,
    MachineAccount = 6,
    Tcb = 7,
    Security = 8,
    TakeOwnership = 9,
    LoadDriver = 10,
    SystemProfile = 11,
    Systemtime = 12,
    ProfileSingleProcess = 13,
    IncreaseBasePriority = 14,
    CreatePagefile = 15,
    CreatePermanent = 16,
    Backup = 17,
    Restore = 18,
    Shutdown = 19,
    Debug = 20,
    Audit = 21,
    SystemEnvironment = 22,
    ChangeNotify = 23,
    RemoteShutdown = 24,
    Undock = 25,
    SyncAgent = 26,
    EnableDelegation = 27,
    ManageVolume = 28,
    Impersonate = 29,
    CreateGlobal = 30,
    TrustedCredManAccess = 31,
    Relabel = 32,
    IncreaseWorkingSet = 33,
    TimeZone = 34,
    CreateSymbolicLink = 35,
}

// LUID (Locally Unique Identifier)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Luid {
    pub low_part: u32,
    pub high_part: i32,
}

impl Luid {
    pub fn new(value: u64) -> Self {
        Self {
            low_part: value as u32,
            high_part: (value >> 32) as i32,
        }
    }
    
    pub fn as_u64(&self) -> u64 {
        ((self.high_part as u64) << 32) | (self.low_part as u64)
    }
}

// Privilege and LUID pair
#[derive(Debug, Clone)]
pub struct LuidAndAttributes {
    pub luid: Luid,
    pub attributes: PrivilegeAttributes,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct PrivilegeAttributes: u32 {
        const SE_PRIVILEGE_ENABLED_BY_DEFAULT = 0x00000001;
        const SE_PRIVILEGE_ENABLED = 0x00000002;
        const SE_PRIVILEGE_REMOVED = 0x00000004;
        const SE_PRIVILEGE_USED_FOR_ACCESS = 0x80000000;
    }
}

// Token types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Primary = 1,
    Impersonation = 2,
}

// Impersonation levels
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityImpersonationLevel {
    Anonymous = 0,
    Identification = 1,
    Impersonation = 2,
    Delegation = 3,
}

// Access Token
#[derive(Debug)]
pub struct Token {
    pub header: ObjectHeader,
    pub token_type: TokenType,
    pub impersonation_level: SecurityImpersonationLevel,
    pub user_sid: Sid,
    pub groups: Vec<(Sid, u32)>, // SID and attributes
    pub privileges: Vec<LuidAndAttributes>,
    pub owner_sid: Sid,
    pub primary_group: Sid,
    pub default_dacl: Option<Acl>,
    pub session_id: u32,
    pub token_id: Luid,
    pub authentication_id: Luid,
    pub expiration_time: u64,
    pub token_flags: TokenFlags,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct TokenFlags: u32 {
        const TOKEN_WRITE_RESTRICTED = 0x0008;
        const TOKEN_IS_RESTRICTED = 0x0010;
        const TOKEN_SESSION_NOT_REFERENCED = 0x0020;
        const TOKEN_SANDBOX_INERT = 0x0040;
        const TOKEN_VIRTUALIZE_ALLOWED = 0x0200;
        const TOKEN_VIRTUALIZE_ENABLED = 0x0400;
        const TOKEN_IS_FILTERED = 0x0800;
        const TOKEN_UIACCESS = 0x1000;
        const TOKEN_NOT_LOW = 0x2000;
        const TOKEN_LOWBOX = 0x4000;
        const TOKEN_HAS_OWN_CLAIM_ATTRIBUTES = 0x8000;
    }
}

impl Token {
    pub fn new(
        token_type: TokenType,
        user_sid: Sid,
        groups: Vec<(Sid, u32)>,
        privileges: Vec<LuidAndAttributes>,
    ) -> Self {
        Self {
            header: ObjectHeader::new(ObjectType::Token),
            token_type,
            impersonation_level: SecurityImpersonationLevel::Anonymous,
            user_sid: user_sid.clone(),
            groups,
            privileges,
            owner_sid: user_sid.clone(),
            primary_group: user_sid,
            default_dacl: None,
            session_id: 0,
            token_id: Luid::new(0),
            authentication_id: Luid::new(0),
            expiration_time: u64::MAX,
            token_flags: TokenFlags::empty(),
        }
    }
    
    pub fn create_system_token() -> Self {
        let system_sid = WellKnownSids::system_sid();
        let groups = vec![
            (WellKnownSids::administrators_sid(), 0x0000000F), // SE_GROUP_ENABLED | SE_GROUP_ENABLED_BY_DEFAULT | SE_GROUP_MANDATORY | SE_GROUP_OWNER
            (WellKnownSids::world_sid(), 0x00000007), // SE_GROUP_ENABLED | SE_GROUP_ENABLED_BY_DEFAULT | SE_GROUP_MANDATORY
        ];
        
        let privileges = vec![
            LuidAndAttributes {
                luid: Luid::new(Privilege::Tcb as u64),
                attributes: PrivilegeAttributes::SE_PRIVILEGE_ENABLED | PrivilegeAttributes::SE_PRIVILEGE_ENABLED_BY_DEFAULT,
            },
            LuidAndAttributes {
                luid: Luid::new(Privilege::Debug as u64),
                attributes: PrivilegeAttributes::SE_PRIVILEGE_ENABLED | PrivilegeAttributes::SE_PRIVILEGE_ENABLED_BY_DEFAULT,
            },
            LuidAndAttributes {
                luid: Luid::new(Privilege::TakeOwnership as u64),
                attributes: PrivilegeAttributes::SE_PRIVILEGE_ENABLED | PrivilegeAttributes::SE_PRIVILEGE_ENABLED_BY_DEFAULT,
            },
        ];
        
        let mut token = Self::new(TokenType::Primary, system_sid, groups, privileges);
        token.session_id = 0;
        token
    }
    
    pub fn has_privilege(&self, privilege: Privilege) -> bool {
        let luid = Luid::new(privilege as u64);
        self.privileges.iter().any(|p| {
            p.luid == luid && p.attributes.contains(PrivilegeAttributes::SE_PRIVILEGE_ENABLED)
        })
    }
    
    pub fn enable_privilege(&mut self, privilege: Privilege) -> NtStatus {
        let luid = Luid::new(privilege as u64);
        for p in &mut self.privileges {
            if p.luid == luid {
                p.attributes.insert(PrivilegeAttributes::SE_PRIVILEGE_ENABLED);
                return NtStatus::Success;
            }
        }
        NtStatus::PrivilegeNotHeld
    }
    
    pub fn disable_privilege(&mut self, privilege: Privilege) -> NtStatus {
        let luid = Luid::new(privilege as u64);
        for p in &mut self.privileges {
            if p.luid == luid {
                p.attributes.remove(PrivilegeAttributes::SE_PRIVILEGE_ENABLED);
                return NtStatus::Success;
            }
        }
        NtStatus::PrivilegeNotHeld
    }
}

impl ObjectTrait for Token {
    fn get_header(&self) -> &ObjectHeader {
        &self.header
    }
    
    fn get_header_mut(&mut self) -> &mut ObjectHeader {
        &mut self.header
    }
}

// Security Manager
pub struct SecurityManager {
    tokens: BTreeMap<Handle, Token>,
    next_luid: AtomicU32,
    audit_enabled: bool,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            tokens: BTreeMap::new(),
            next_luid: AtomicU32::new(1000),
            audit_enabled: false,
        }
    }
    
    pub fn initialize(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("Security: Initializing Windows security subsystem");
        
        // Create system token
        let system_token = Token::create_system_token();
        let system_handle = Handle::new();
        self.tokens.insert(system_handle, system_token);
        
        serial_println!("Security: Created SYSTEM token");
        serial_println!("Security: Security subsystem initialized");
        
        NtStatus::Success
    }
    
    pub fn create_token(
        &mut self,
        token_type: TokenType,
        user_sid: Sid,
        groups: Vec<(Sid, u32)>,
        privileges: Vec<LuidAndAttributes>,
    ) -> Result<Handle, NtStatus> {
        let mut token = Token::new(token_type, user_sid, groups, privileges);
        token.token_id = Luid::new(self.next_luid.fetch_add(1, Ordering::SeqCst) as u64);
        token.authentication_id = token.token_id;
        
        let handle = Handle::new();
        self.tokens.insert(handle, token);
        
        Ok(handle)
    }
    
    pub fn duplicate_token(
        &mut self,
        existing_token: Handle,
        token_type: TokenType,
        impersonation_level: SecurityImpersonationLevel,
    ) -> Result<Handle, NtStatus> {
        let original = self.tokens.get(&existing_token)
            .ok_or(NtStatus::InvalidHandle)?;
        
        let mut new_token = Token::new(
            token_type,
            original.user_sid.clone(),
            original.groups.clone(),
            original.privileges.clone(),
        );
        
        new_token.impersonation_level = impersonation_level;
        new_token.owner_sid = original.owner_sid.clone();
        new_token.primary_group = original.primary_group.clone();
        new_token.default_dacl = original.default_dacl.clone();
        new_token.session_id = original.session_id;
        new_token.token_id = Luid::new(self.next_luid.fetch_add(1, Ordering::SeqCst) as u64);
        new_token.authentication_id = original.authentication_id;
        
        let handle = Handle::new();
        self.tokens.insert(handle, new_token);
        
        Ok(handle)
    }
    
    pub fn access_check(
        &self,
        security_descriptor: &SecurityDescriptor,
        token_handle: Handle,
        desired_access: u32,
        generic_mapping: &GenericMapping,
    ) -> Result<u32, NtStatus> {
        let token = self.tokens.get(&token_handle)
            .ok_or(NtStatus::InvalidHandle)?;
        
        // Map generic rights to specific rights
        let specific_access = self.map_generic_access(desired_access, generic_mapping);
        
        // Check owner - owner always has full access
        if let Some(ref owner_sid) = security_descriptor.owner_sid {
            if *owner_sid == token.user_sid {
                return Ok(specific_access);
            }
        }
        
        // Check DACL
        if let Some(ref dacl) = security_descriptor.dacl {
            if dacl.check_access(&token.user_sid, specific_access) {
                return Ok(specific_access);
            }
            
            // Check group SIDs
            for (group_sid, _) in &token.groups {
                if dacl.check_access(group_sid, specific_access) {
                    return Ok(specific_access);
                }
            }
        }
        
        Err(NtStatus::AccessDenied)
    }
    
    fn map_generic_access(&self, access: u32, mapping: &GenericMapping) -> u32 {
        let mut specific = access;
        
        if access & GENERIC_READ != 0 {
            specific = (specific & !GENERIC_READ) | mapping.generic_read;
        }
        if access & GENERIC_WRITE != 0 {
            specific = (specific & !GENERIC_WRITE) | mapping.generic_write;
        }
        if access & GENERIC_EXECUTE != 0 {
            specific = (specific & !GENERIC_EXECUTE) | mapping.generic_execute;
        }
        if access & GENERIC_ALL != 0 {
            specific = (specific & !GENERIC_ALL) | mapping.generic_all;
        }
        
        specific
    }
    
    pub fn get_token_user(&self, token_handle: Handle) -> Result<Sid, NtStatus> {
        self.tokens.get(&token_handle)
            .map(|token| token.user_sid.clone())
            .ok_or(NtStatus::InvalidHandle)
    }
    
    pub fn get_token_groups(&self, token_handle: Handle) -> Result<Vec<(Sid, u32)>, NtStatus> {
        self.tokens.get(&token_handle)
            .map(|token| token.groups.clone())
            .ok_or(NtStatus::InvalidHandle)
    }
    
    pub fn get_token_privileges(&self, token_handle: Handle) -> Result<Vec<LuidAndAttributes>, NtStatus> {
        self.tokens.get(&token_handle)
            .map(|token| token.privileges.clone())
            .ok_or(NtStatus::InvalidHandle)
    }
    
    pub fn adjust_token_privileges(
        &mut self,
        token_handle: Handle,
        disable_all: bool,
        new_state: Vec<LuidAndAttributes>,
    ) -> NtStatus {
        if let Some(token) = self.tokens.get_mut(&token_handle) {
            if disable_all {
                for privilege in &mut token.privileges {
                    privilege.attributes.remove(PrivilegeAttributes::SE_PRIVILEGE_ENABLED);
                }
            }
            
            for new_priv in new_state {
                for existing_priv in &mut token.privileges {
                    if existing_priv.luid == new_priv.luid {
                        existing_priv.attributes = new_priv.attributes;
                        break;
                    }
                }
            }
            
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }
}

// Generic access rights mapping
pub struct GenericMapping {
    pub generic_read: u32,
    pub generic_write: u32,
    pub generic_execute: u32,
    pub generic_all: u32,
}

// Standard access rights
pub const DELETE: u32 = 0x00010000;
pub const READ_CONTROL: u32 = 0x00020000;
pub const WRITE_DAC: u32 = 0x00040000;
pub const WRITE_OWNER: u32 = 0x00080000;
pub const SYNCHRONIZE: u32 = 0x00100000;

pub const STANDARD_RIGHTS_REQUIRED: u32 = 0x000F0000;
pub const STANDARD_RIGHTS_READ: u32 = READ_CONTROL;
pub const STANDARD_RIGHTS_WRITE: u32 = READ_CONTROL;
pub const STANDARD_RIGHTS_EXECUTE: u32 = READ_CONTROL;
pub const STANDARD_RIGHTS_ALL: u32 = 0x001F0000;

// Generic access rights
pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;
pub const GENERIC_EXECUTE: u32 = 0x20000000;
pub const GENERIC_ALL: u32 = 0x10000000;

// File access rights
pub const FILE_READ_DATA: u32 = 0x00000001;
pub const FILE_WRITE_DATA: u32 = 0x00000002;
pub const FILE_APPEND_DATA: u32 = 0x00000004;
pub const FILE_READ_EA: u32 = 0x00000008;
pub const FILE_WRITE_EA: u32 = 0x00000010;
pub const FILE_EXECUTE: u32 = 0x00000020;
pub const FILE_DELETE_CHILD: u32 = 0x00000040;
pub const FILE_READ_ATTRIBUTES: u32 = 0x00000080;
pub const FILE_WRITE_ATTRIBUTES: u32 = 0x00000100;

pub const FILE_ALL_ACCESS: u32 = STANDARD_RIGHTS_REQUIRED | SYNCHRONIZE | 0x1FF;
pub const FILE_GENERIC_READ: u32 = STANDARD_RIGHTS_READ | FILE_READ_DATA | FILE_READ_ATTRIBUTES | FILE_READ_EA | SYNCHRONIZE;
pub const FILE_GENERIC_WRITE: u32 = STANDARD_RIGHTS_WRITE | FILE_WRITE_DATA | FILE_WRITE_ATTRIBUTES | FILE_WRITE_EA | FILE_APPEND_DATA | SYNCHRONIZE;
pub const FILE_GENERIC_EXECUTE: u32 = STANDARD_RIGHTS_EXECUTE | FILE_READ_ATTRIBUTES | FILE_EXECUTE | SYNCHRONIZE;

// Global security manager
lazy_static! {
    pub static ref SECURITY_MANAGER: Mutex<SecurityManager> = Mutex::new(SecurityManager::new());
}

// Public API functions
pub fn initialize_security() -> NtStatus {
    let mut manager = SECURITY_MANAGER.lock();
    manager.initialize()
}

pub fn nt_create_token(
    token_handle: &mut Handle,
    desired_access: u32,
    object_attributes: Option<&super::object::ObjectAttributes>,
    token_type: TokenType,
    authentication_id: &Luid,
    expiration_time: &u64,
    user: &Sid,
    groups: &[(Sid, u32)],
    privileges: &[LuidAndAttributes],
    owner: Option<&Sid>,
    primary_group: &Sid,
    default_dacl: Option<&Acl>,
) -> NtStatus {
    let mut manager = SECURITY_MANAGER.lock();
    
    match manager.create_token(
        token_type,
        user.clone(),
        groups.to_vec(),
        privileges.to_vec(),
    ) {
        Ok(handle) => {
            *token_handle = handle;
            
            // Set additional token properties
            if let Some(token) = manager.tokens.get_mut(&handle) {
                token.authentication_id = *authentication_id;
                token.expiration_time = *expiration_time;
                if let Some(owner_sid) = owner {
                    token.owner_sid = owner_sid.clone();
                }
                token.primary_group = primary_group.clone();
                if let Some(dacl) = default_dacl {
                    token.default_dacl = Some(dacl.clone());
                }
            }
            
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_duplicate_token(
    existing_token: Handle,
    desired_access: u32,
    object_attributes: Option<&super::object::ObjectAttributes>,
    effective_only: bool,
    token_type: TokenType,
    new_token: &mut Handle,
) -> NtStatus {
    let mut manager = SECURITY_MANAGER.lock();
    
    match manager.duplicate_token(
        existing_token,
        token_type,
        SecurityImpersonationLevel::Impersonation,
    ) {
        Ok(handle) => {
            *new_token = handle;
            NtStatus::Success
        }
        Err(status) => status,
    }
}

pub fn nt_adjust_privileges_token(
    token_handle: Handle,
    disable_all_privileges: bool,
    new_state: Option<&[LuidAndAttributes]>,
    buffer_length: u32,
    previous_state: Option<&mut [LuidAndAttributes]>,
    return_length: Option<&mut u32>,
) -> NtStatus {
    let mut manager = SECURITY_MANAGER.lock();
    
    manager.adjust_token_privileges(
        token_handle,
        disable_all_privileges,
        new_state.map(|s| s.to_vec()).unwrap_or_default(),
    )
}

pub fn nt_access_check(
    security_descriptor: &SecurityDescriptor,
    client_token: Handle,
    desired_access: u32,
    generic_mapping: &GenericMapping,
    privileges: Option<&mut Vec<LuidAndAttributes>>,
    granted_access: &mut u32,
    access_status: &mut NtStatus,
) -> NtStatus {
    let manager = SECURITY_MANAGER.lock();
    
    match manager.access_check(security_descriptor, client_token, desired_access, generic_mapping) {
        Ok(access) => {
            *granted_access = access;
            *access_status = NtStatus::Success;
            NtStatus::Success
        }
        Err(status) => {
            *granted_access = 0;
            *access_status = status;
            NtStatus::Success // Function succeeded, but access was denied
        }
    }
}
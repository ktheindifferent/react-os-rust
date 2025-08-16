pub mod object;
pub mod process;
pub mod thread;
pub mod syscall;
pub mod executive;
pub mod exception;
pub mod registry;
pub mod security;
pub mod network;
pub mod activation;
// pub mod io;
// pub mod drivers;
// pub mod filesystem;
// pub mod pe_loader;

// NT Status codes - compatible with Windows NT
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtStatus {
    Success = 0x00000000,
    Unsuccessful = 0xC0000001,
    NotImplemented = 0xC0000002,
    InvalidInfoClass = 0xC0000003,
    InfoLengthMismatch = 0xC0000004,
    AccessViolation = 0xC0000005,
    InPageError = 0xC0000006,
    PagefileQuota = 0xC0000007,
    InvalidHandle = 0xC0000008,
    BadInitialStack = 0xC0000009,
    BadInitialPc = 0xC000000A,
    InvalidCid = 0xC000000B,
    TimerNotCanceled = 0xC000000C,
    InvalidParameter = 0xC000000D,
    NoSuchDevice = 0xC000000E,
    NoSuchFile = 0xC000000F,
    InvalidDeviceRequest = 0xC0000010,
    EndOfFile = 0xC0000011,
    WrongVolume = 0xC0000012,
    NoMediaInDevice = 0xC0000013,
    NoMemory = 0xC0000017,
    NotMappedView = 0xC0000019,
    UnableToFreeVm = 0xC000001A,
    UnableToDeleteSection = 0xC000001B,
    IllegalInstruction = 0xC000001D,
    AlreadyCommitted = 0xC0000021,
    AccessDenied = 0xC0000022,
    BufferTooSmall = 0xC0000023,
    ObjectTypeMismatch = 0xC0000024,
    NonContinuableException = 0xC0000025,
    BadStack = 0xC0000028,
    NotLocked = 0xC000002A,
    NotCommitted = 0xC000002D,
    InvalidParameterMix = 0xC0000030,
    ObjectNameInvalid = 0xC0000033,
    ObjectNameNotFound = 0xC0000034,
    ObjectNameCollision = 0xC0000035,
    ObjectPathInvalid = 0xC0000039,
    ObjectPathNotFound = 0xC000003A,
    ObjectPathSyntaxBad = 0xC000003B,
    DataOverrun = 0xC000003C,
    DataLate = 0xC000003D,
    DataError = 0xC000003E,
    CrcError = 0xC000003F,
    SectionTooLarge = 0xC0000040,
    PortConnectionRefused = 0xC0000041,
    InvalidPortHandle = 0xC0000042,
    SharingViolation = 0xC0000043,
    QuotaExceeded = 0xC0000044,
    InvalidPageProtection = 0xC0000045,
    MutantNotOwned = 0xC0000046,
    SemaphoreLimitExceeded = 0xC0000047,
    PortAlreadySet = 0xC0000048,
    SectionNotImage = 0xC0000049,
    SuspendCountExceeded = 0xC000004A,
    ThreadIsTerminating = 0xC000004B,
    BadWorkingSetLimit = 0xC000004C,
    IncompatibleFileMap = 0xC000004D,
    SectionProtection = 0xC000004E,
    EasNotSupported = 0xC000004F,
    EaTooLarge = 0xC0000050,
    NonExistentEaEntry = 0xC0000051,
    NoEasOnFile = 0xC0000052,
    EaCorruptError = 0xC0000053,
    FileLockConflict = 0xC0000054,
    LockNotGranted = 0xC0000055,
    DeletePending = 0xC0000056,
    CtlFileNotSupported = 0xC0000057,
    UnknownRevision = 0xC0000058,
    RevisionMismatch = 0xC0000059,
    InvalidOwner = 0xC000005A,
    InvalidPrimaryGroup = 0xC000005B,
    NoImpersonationToken = 0xC000005C,
    CantDisableMandatory = 0xC000005D,
    NoLogonServers = 0xC000005E,
    NoSuchLogonSession = 0xC000005F,
    NoSuchPrivilege = 0xC0000060,
    PrivilegeNotHeld = 0xC0000061,
    InvalidAccountName = 0xC0000062,
    UserExists = 0xC0000063,
    NoSuchUser = 0xC0000064,
    GroupExists = 0xC0000065,
    NoSuchGroup = 0xC0000066,
    MemberInGroup = 0xC0000067,
    MemberNotInGroup = 0xC0000068,
    LastAdmin = 0xC0000069,
    WrongPassword = 0xC000006A,
    IllFormedPassword = 0xC000006B,
    PasswordRestriction = 0xC000006C,
    LogonFailure = 0xC000006D,
    AccountRestriction = 0xC000006E,
    InvalidLogonHours = 0xC000006F,
    InvalidWorkstation = 0xC0000070,
    PasswordExpired = 0xC0000071,
    AccountDisabled = 0xC0000072,
    NoneMapped = 0xC0000073,
    TooManyLuidsRequested = 0xC0000074,
    LuidsExhausted = 0xC0000075,
    InvalidSubAuthority = 0xC0000076,
    InvalidAcl = 0xC0000077,
    InvalidSid = 0xC0000078,
    InvalidSecurityDescr = 0xC0000079,
    ProcedureNotFound = 0xC000007A,
    InvalidImageFormat = 0xC000007B,
    NoToken = 0xC000007C,
    BadInheritanceAcl = 0xC000007D,
    RangeNotLocked = 0xC000007E,
    DiskFull = 0xC000007F,
    ServerDisabled = 0xC0000080,
    ServerNotDisabled = 0xC0000081,
    TooManyGuidsRequested = 0xC0000082,
    GuidsExhausted = 0xC0000083,
    InvalidIdAuthority = 0xC0000084,
    AgentsExhausted = 0xC0000085,
    InvalidVolumeLabel = 0xC0000086,
    SectionNotExtended = 0xC0000087,
    NotMappedData = 0xC0000088,
    ResourceDataNotFound = 0xC0000089,
    ResourceTypeNotFound = 0xC000008A,
    ResourceNameNotFound = 0xC000008B,
    ArrayBoundsExceeded = 0xC000008C,
    FloatDenormalOperand = 0xC000008D,
    FloatDivideByZero = 0xC000008E,
    FloatInexactResult = 0xC000008F,
    FloatInvalidOperation = 0xC0000090,
    FloatOverflow = 0xC0000091,
    FloatStackCheck = 0xC0000092,
    FloatUnderflow = 0xC0000093,
    IntegerDivideByZero = 0xC0000094,
    IntegerOverflow = 0xC0000095,
    PrivilegedInstruction = 0xC0000096,
    TooManyPagingFiles = 0xC0000097,
    FileInvalid = 0xC0000098,
    InsufficientResources = 0xC000009A,
    DeviceNotReady = 0xC00000A3,
    MediaWriteProtected = 0xC00000A2,
    InstanceNotAvailable = 0xC00000AB,
    PipeNotAvailable = 0xC00000AC,
    InvalidPipeState = 0xC00000AD,
    PipeBusy = 0xC00000AE,
    IllegalFunction = 0xC00000AF,
    PipeDisconnected = 0xC00000B0,
    PipeClosing = 0xC00000B1,
    PipeConnected = 0xC00000B2,
    PipeListening = 0xC00000B3,
    InvalidReadMode = 0xC00000B4,
    IoTimeout = 0xC00000B5,
    FileForcedClosed = 0xC00000B6,
    ProfilingNotStarted = 0xC00000B7,
    ProfilingNotStopped = 0xC00000B8,
    NotSameDevice = 0xC00000D4,
    FileRenamed = 0xC00000D5,
    CantWait = 0xC00000D8,
    PipeEmpty = 0xC00000D9,
    CantTerminateSelf = 0xC00000DB,
    InternalError = 0xC00000E5,
    InvalidParameter1 = 0xC00000EF,
    InvalidParameter2 = 0xC00000F0,
    InvalidParameter3 = 0xC00000F1,
    InvalidParameter4 = 0xC00000F2,
    InvalidParameter5 = 0xC00000F3,
    InvalidParameter6 = 0xC00000F4,
    InvalidParameter7 = 0xC00000F5,
    InvalidParameter8 = 0xC00000F6,
    InvalidParameter9 = 0xC00000F7,
    InvalidParameter10 = 0xC00000F8,
    InvalidParameter11 = 0xC00000F9,
    InvalidParameter12 = 0xC00000FA,
    MappedFileSizeZero = 0xC000011E,
    TooManyOpenedFiles = 0xC000011F,
    Cancelled = 0xC0000120,
    CannotDelete = 0xC0000121,
    InvalidComputerName = 0xC0000122,
    FileDeleted = 0xC0000123,
    SpecialAccount = 0xC0000124,
    SpecialGroup = 0xC0000125,
    SpecialUser = 0xC0000126,
    MembersPrimaryGroup = 0xC0000127,
    FileClosed = 0xC0000128,
    TooManyThreads = 0xC0000129,
    ThreadNotInProcess = 0xC000012A,
    TokenAlreadyInUse = 0xC000012B,
    PagefileQuotaExceeded = 0xC000012C,
    CommitmentLimit = 0xC000012D,
    InvalidImageLeFormat = 0xC000012E,
    InvalidImageNotMz = 0xC000012F,
    InvalidImageProtect = 0xC0000130,
    InvalidImageWin16 = 0xC0000131,
    LogonServer = 0xC0000132,
    DifferenceAtDc = 0xC0000133,
    SynchronizationRequired = 0xC0000134,
    DllNotFound = 0xC0000135,
    IoPrivilegeFailed = 0xC0000137,
    OrdinalNotFound = 0xC0000138,
    EntryPointNotFound = 0xC0000139,
    ControlCExit = 0xC000013A,
    PortNotSet = 0xC0000353,
    DebuggerInactive = 0xC0000354,
    CallbackBypass = 0xC0000503,
    PortClosed = 0xC0000700,
    MessageLost = 0xC0000701,
    InvalidMessage = 0xC0000702,
    RequestCanceled = 0xC0000703,
    RecursiveDispatch = 0xC0000704,
    LpcReceiveBufferExpected = 0xC0000705,
    LpcInvalidConnectionUsage = 0xC0000706,
    LpcRequestsNotAllowed = 0xC0000707,
    ResourceInUse = 0xC0000708,
    ProcessIsProtected = 0xC0000712,
    VolumeDirty = 0xC0000806,
    FileCheckedOut = 0xC0000901,
    CheckOutRequired = 0xC0000902,
    BadFileType = 0xC0000903,
    FileTooLarge = 0xC0000904,
    FormsAuthRequired = 0xC0000905,
    VirusInfected = 0xC0000906,
    VirusDeleted = 0xC0000907,
    TransactionalConflict = 0xC0190001,
    InvalidTransaction = 0xC0190002,
    TransactionNotActive = 0xC0190003,
    TmInitializationFailed = 0xC0190004,
    RmNotActive = 0xC0190005,
    RmMetadataCorrupt = 0xC0190006,
    TransactionNotJoined = 0xC0190007,
    DirectoryNotRm = 0xC0190008,
    CouldNotResizeLog = 0xC0190009,
    TransactionsUnsupportedRemote = 0xC019000A,
    LogResizeInvalidSize = 0xC019000B,
    RemoteFileVersionMismatch = 0xC019000C,
    CrmProtocolAlreadyExists = 0xC019000F,
    TransactionPropagationFailed = 0xC0190010,
    CrmProtocolNotFound = 0xC0190011,
    TransactionSuperiorExists = 0xC0190012,
    TransactionRequestNotValid = 0xC0190013,
    TransactionNotRequested = 0xC0190014,
    TransactionAlreadyAborted = 0xC0190015,
    TransactionAlreadyCommitted = 0xC0190016,
    TransactionInvalidMarshallBuffer = 0xC0190017,
    CurrentTransactionNotValid = 0xC0190018,
    LogGrowthFailed = 0xC0190019,
    ObjectNoLongerExists = 0xC0190021,
    StreamMiniversionNotFound = 0xC0190022,
    StreamMiniversionNotValid = 0xC0190023,
    MiniversionInaccessibleFromSpecifiedTransaction = 0xC0190024,
    CantOpenMiniversionWithModifyIntent = 0xC0190025,
    CantCreateMoreStreamMiniversions = 0xC0190026,
    HandleNoLongerValid = 0xC0190028,
    NoTxfMetadata = 0xC0190029,
    LogCorruptionDetected = 0xC0190030,
    CantRecoverWithHandleOpen = 0xC0190031,
    RmDisconnected = 0xC0190032,
    EnlistmentNotSuperior = 0xC0190033,
    RecoveryNotNeeded = 0xC0190034,
    RmAlreadyStarted = 0xC0190035,
    FileIdentityNotPersistent = 0xC0190036,
    CantBreakTransactionalDependency = 0xC0190037,
    CantCrossRmBoundary = 0xC0190038,
    TxfDirNotEmpty = 0xC0190039,
    IndoubtTransactionsExist = 0xC019003A,
    TmVolatile = 0xC019003B,
    RollbackTimerExpired = 0xC019003C,
    TxfAttributeCorrupt = 0xC019003D,
    EfsNotAllowedInTransaction = 0xC019003E,
    TransactionalOpenNotAllowed = 0xC019003F,
    TransactedMappingUnsupportedRemote = 0xC0190040,
    TxfMetadataAlreadyPresent = 0xC0190041,
    TransactionScopeCallbacksNotSet = 0xC0190042,
    TransactionRequiredPromotion = 0xC0190043,
    CannotExecuteFileInTransaction = 0xC0190044,
    TransactionsNotFrozen = 0xC0190045,
    NoMoreEntries = 0x8000001A,
    InvalidDeviceState = 0xC0000184,
    LicenseViolation = 0xC0000190,

    MaximumNtStatus = 0xFFFFFFFF
}

impl From<u32> for NtStatus {
    fn from(value: u32) -> Self {
        // For simplicity, we'll just return Success for unknown values
        // In a real implementation, we'd have a proper conversion
        if value == 0 {
            NtStatus::Success
        } else {
            NtStatus::Unsuccessful
        }
    }
}

impl Into<u32> for NtStatus {
    fn into(self) -> u32 {
        self as u32
    }
}

// Helper macros for NT status checking
#[macro_export]
macro_rules! nt_success {
    ($status:expr) => {
        ($status as u32) >= 0 && ($status as u32) <= 0x3FFFFFFF
    };
}

#[macro_export]
macro_rules! nt_information {
    ($status:expr) => {
        ($status as u32) >= 0x40000000 && ($status as u32) <= 0x7FFFFFFF
    };
}

#[macro_export]
macro_rules! nt_warning {
    ($status:expr) => {
        ($status as u32) >= 0x80000000 && ($status as u32) <= 0xBFFFFFFF
    };
}

#[macro_export]
macro_rules! nt_error {
    ($status:expr) => {
        ($status as u32) >= 0xC0000000
    };
}
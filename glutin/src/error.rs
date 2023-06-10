//! Glutin error handling.

use std::fmt;

/// A specialized [`Result`] type for graphics operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for all the graphics platform operations.
#[derive(Debug, Clone)]
pub struct Error {
    /// The raw code of the underlying error.
    raw_code: Option<i64>,

    /// The raw message from the os in case it could be obtained.
    raw_os_message: Option<String>,

    /// The simplified error kind to handle mathing.
    kind: ErrorKind,
}

impl Error {
    #[allow(dead_code)]
    pub(crate) fn new(
        raw_code: Option<i64>,
        raw_os_message: Option<String>,
        kind: ErrorKind,
    ) -> Self {
        Self { raw_code, raw_os_message, kind }
    }

    /// Helper to check that error is [`ErrorKind::NotSupported`].
    #[inline]
    pub fn not_supported(&self) -> bool {
        matches!(&self.kind, ErrorKind::NotSupported(_))
    }

    /// The underlying error kind.
    #[inline]
    pub fn error_kind(&self) -> ErrorKind {
        self.kind
    }

    /// The underlying raw code in case it's present.
    #[inline]
    pub fn raw_code(&self) -> Option<i64> {
        self.raw_code
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(raw_code) = self.raw_code {
            write!(f, "[{raw_code:x}] ")?;
        }

        let msg = if let Some(raw_os_message) = self.raw_os_message.as_ref() {
            raw_os_message
        } else {
            self.kind.as_str()
        };

        write!(f, "{msg}")
    }
}

impl std::error::Error for Error {}

/// Build an error with just a kind.
impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error { raw_code: None, raw_os_message: None, kind }
    }
}

/// A list specifying general categoires of native platform graphics interface
/// errors.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ErrorKind {
    /// The requested display wasn't found or some required symbol in it was
    /// missing.
    NotFound,

    /// Failed to perform resource initialization.
    InitializationFailed,

    /// Can't access a requested resource.
    ///
    /// For example when trying to make a context current while it's current on
    /// another thread.
    BadAccess,

    /// An operation could not be completed, because it failed to allocate
    /// enough memory.
    OutOfMemory,

    /// An recognized attribute value was passed.
    BadAttribute,

    /// The context is no longer valid.
    BadContext,

    /// The context is in bad state.
    BadContextState,

    /// Invalid config was passed.
    BadConfig,

    /// The current surface of the calling thread is no longer valid.
    BadCurrentSurface,

    /// The display is no longer valid.
    BadDisplay,

    /// The surface is invalid.
    BadSurface,

    /// The pbuffer is invalid.
    BadPbuffer,

    /// The pixmap is invalid.
    BadPixmap,

    /// Arguments are inconsistent. For example when shared contexts are not
    /// compatible.
    BadMatch,

    /// One or more argument values are invalid.
    BadParameter,

    /// Bad native pixmap was provided.
    BadNativePixmap,

    /// Bad native window was provided.
    BadNativeWindow,

    /// The context was lost.
    ContextLost,

    /// The operation is not supported by the platform.
    NotSupported(&'static str),

    /// The misc error that can't be classified occurred.
    Misc,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        match *self {
            NotFound => "not found",
            InitializationFailed => "initialization failed",
            BadAccess => "access to the resource failed",
            OutOfMemory => "out of memory",
            BadAttribute => "an anrecougnized attribute or attribute value was passed",
            BadContext => "argument does not name a valid context",
            BadContextState => "the context is in a bad state",
            BadConfig => "argument does not name a valid config",
            BadCurrentSurface => "the current surface of the calling thread is no longer valid",
            BadDisplay => "argument does not name a valid display",
            BadSurface => "argument does not name a valid surface",
            BadPbuffer => "argument does not name a valid pbuffer",
            BadPixmap => "argument does not name a valid pixmap",
            BadMatch => "arguments are inconsistance",
            BadParameter => "one or more argument values are invalid",
            BadNativePixmap => "argument does not refer to a valid native pixmap",
            BadNativeWindow => "argument does not refer to a valid native window",
            ContextLost => "context loss",
            NotSupported(reason) => reason,
            Misc => "misc platform error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

use crate::prelude::*;
use core::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum SalmiakErrorKind {
    InitCPUError(String),

    InitGPUError(String),
    InitSerialError(String),
}

#[derive(Debug)]
pub struct SalmiakError {
    kind: SalmiakErrorKind,
}

impl SalmiakError {
    pub fn kind(&self) -> &SalmiakErrorKind {
        &self.kind
    }
}

impl From<SalmiakErrorKind> for SalmiakError {
    fn from(kind: SalmiakErrorKind) -> Self {
        SalmiakError { kind }
    }
}

impl Display for SalmiakError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            SalmiakErrorKind::InitCPUError(mess) => write!(f, "{}", mess),
            SalmiakErrorKind::InitGPUError(mess) => write!(f, "{}", mess),
            SalmiakErrorKind::InitSerialError(mess) => write!(f, "{}", mess),
        }
    }
}

use eyre::eyre;
use std::convert::Infallible;
use std::ffi::OsString;
use std::path::PathBuf;
use widestring::U16CString;
use windows::core::PCWSTR;
use windows::core::Param;

pub struct PCWSTRGuard {
    string: U16CString,
}
impl PCWSTRGuard {
    pub fn new(string: U16CString) -> Self {
        Self { string }
    }

    /// # Safety
    ///
    /// You must ensure that the `PCWSTRGuard` outlives any usage of the pointer.
    pub unsafe fn as_ptr(&self) -> PCWSTR {
        PCWSTR(self.string.as_ptr())
    }
}
// MUST NOT implement this for `PCWSTRGuard` itself, only for `&PCWSTRGuard`, to ensure the data the PCWSTR points to is valid for the lifetime of the parameter.
impl Param<PCWSTR> for &PCWSTRGuard {
    unsafe fn param(self) -> windows::core::ParamValue<PCWSTR> {
        windows::core::ParamValue::Borrowed(PCWSTR(self.string.as_ptr()))
    }
}

impl AsRef<PCWSTRGuard> for PCWSTRGuard {
    fn as_ref(&self) -> &PCWSTRGuard {
        self
    }
}

pub trait EasyPCWSTR {
    type Error;
    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error>;
}
impl EasyPCWSTR for U16CString {
    type Error = Infallible;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(self))
    }
}
impl EasyPCWSTR for &str {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_str(self).map_err(
            |_| eyre!("Failed to convert string to U16CString: {}", self),
        )?))
    }
}
impl EasyPCWSTR for String {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_str(&self).map_err(
            |_| eyre!("Failed to convert string to U16CString: {}", self),
        )?))
    }
}
impl EasyPCWSTR for OsString {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_os_str_truncate(&self)))
    }
}
impl EasyPCWSTR for PathBuf {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_os_str_truncate(
            self.as_os_str(),
        )))
    }
}

#[cfg(test)]
mod test {
    use super::EasyPCWSTR;
    use std::ffi::OsString;

    #[test]
    fn it_works() -> eyre::Result<()> {
        "Hello, World!".easy_pcwstr()?;
        OsString::from("asd").easy_pcwstr()?;
        "asd".to_string().easy_pcwstr()?;
        Ok(())
    }
}

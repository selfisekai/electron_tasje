//! it's not surprising if your targets are not here,
//! what's more surprising is that you're trying to use this code.
//!
//! in the inevitable case this is not covering every use case,
//! extend it, and feel free to send a pull request :)

use anyhow::{bail, Result};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    X86,
    Aarch64,
    ArmV7,
}

impl Architecture {
    pub fn from_tasje_name<N>(name: N) -> Result<Architecture>
    where
        N: AsRef<str>,
    {
        use Architecture::*;
        match name.as_ref() {
            "x86_64" => Ok(X86_64),
            "x86" => Ok(X86),
            "aarch64" => Ok(Aarch64),
            "armv7" => Ok(ArmV7),
            n => bail!("unknown architecture name: {n:?}"),
        }
    }

    pub fn to_node(&self) -> &'static str {
        use Architecture::*;
        match self {
            X86_64 => "x64",
            X86 => "ia32",
            Aarch64 => "arm64",
            ArmV7 => "arm",
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub static HOST_ARCHITECTURE: Architecture = Architecture::X86_64;

#[cfg(target_arch = "x86")]
pub static HOST_ARCHITECTURE: Architecture = Architecture::X86;

#[cfg(target_arch = "aarch64")]
pub static HOST_ARCHITECTURE: Architecture = Architecture::Aarch64;

#[cfg(target_arch = "arm")]
pub static HOST_ARCHITECTURE: Architecture = Architecture::ArmV7;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Windows,
    Darwin,
}

impl Platform {
    pub fn from_tasje_name<N>(name: N) -> Result<Platform>
    where
        N: AsRef<str>,
    {
        use Platform::*;
        match name.as_ref() {
            "linux" => Ok(Linux),
            "windows" => Ok(Windows),
            "darwin" => Ok(Darwin),
            n => bail!("unknown platform name: {n:?}"),
        }
    }

    pub fn to_node(&self) -> &'static str {
        use Platform::*;
        match self {
            Linux => "linux",
            Windows => "win32",
            Darwin => "darwin",
        }
    }
}

#[cfg(target_os = "linux")]
pub static HOST_PLATFORM: Platform = Platform::Linux;

#[cfg(target_os = "windows")]
pub static HOST_PLATFORM: Platform = Platform::Windows;

#[cfg(target_os = "macos")]
pub static HOST_PLATFORM: Platform = Platform::Darwin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Environment {
    pub architecture: Architecture,
    pub platform: Platform,
}

pub static HOST_ENVIRONMENT: Environment = Environment {
    architecture: HOST_ARCHITECTURE,
    platform: HOST_PLATFORM,
};

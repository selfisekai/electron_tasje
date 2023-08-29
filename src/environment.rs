//! it's not surprising if your targets are not here,
//! what's more surprising is that you're trying to use this code.
//!
//! in the inevitable case this is not covering every use case,
//! extend it, and feel free to send a pull request :)

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    X86,
    Aarch64,
    Arm,
}

impl Architecture {
    pub fn to_node(&self) -> &'static str {
        use Architecture::*;
        match self {
            X86_64 => "x64",
            X86 => "ia32",
            Aarch64 => "arm64",
            Arm => "arm",
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub static HOST_ARCHITECTURE: Architecture = Architecture::X86_64;

#[cfg(any(target_arch = "i586", target_arch = "i686"))]
pub static HOST_ARCHITECTURE: Architecture = Architecture::X86;

#[cfg(target_arch = "aarch64")]
pub static HOST_ARCHITECTURE: Architecture = Architecture::Aarch64;

#[cfg(any(target_arch = "armv6", target_arch = "armv7"))]
pub static HOST_ARCHITECTURE: Architecture = Architecture::Arm;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Windows,
    Darwin,
}

impl Platform {
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

#[cfg(target_os = "darwin")]
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

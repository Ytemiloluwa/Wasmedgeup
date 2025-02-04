use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OS {
    Linux(LinuxDistro),
    Darwin,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxDistro {
    Ubuntu,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Arm64,
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Unsupported operating system: {0}")]
    UnsupportedOS(String),
    #[error("Unsupported architecture: {0}")]
    UnsupportedArch(String),
    #[error("Failed to detect system information: {0}")]
    DetectionError(String),
}

impl fmt::Display for OS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OS::Linux(distro) => match distro {
                LinuxDistro::Ubuntu => write!(f, "Ubuntu"),
                LinuxDistro::Generic => write!(f, "Linux"),
            },
            OS::Darwin => write!(f, "Darwin"),
            OS::Windows => write!(f, "Windows"),
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Architecture::X86_64 => write!(f, "x86_64"),
            Architecture::Aarch64 | Architecture::Arm64 => write!(f, "arm64"),
        }
    }
}

impl Architecture {
    pub fn detect() -> Result<Self, PlatformError> {
        let arch = std::env::consts::ARCH;
        match arch {
            "x86_64" => Ok(Architecture::X86_64),
            "aarch64" | "arm64" => Ok(Architecture::Aarch64),
            arch => Err(PlatformError::UnsupportedArch(arch.to_string())),
        }
    }

    pub fn from_str(arch: &str) -> Result<Self, PlatformError> {
        match arch.to_lowercase().as_str() {
            "x86_64" | "amd64" => Ok(Architecture::X86_64),
            "aarch64" | "arm64" => Ok(Architecture::Aarch64),
            arch => Err(PlatformError::UnsupportedArch(arch.to_string())),
        }
    }
}

impl OS {
    pub fn detect() -> Result<Self, PlatformError> {
        let os = std::env::consts::OS;
        
        match os {
            "linux" => {
                // Check for Ubuntu specifically
                if let Ok(release) = std::fs::read_to_string("/etc/os-release") {
                    if release.contains("Ubuntu") {
                        return Ok(OS::Linux(LinuxDistro::Ubuntu));
                    }
                }
                Ok(OS::Linux(LinuxDistro::Generic))
            }
            "macos" => Ok(OS::Darwin),
            "windows" => Ok(OS::Windows),
            os => Err(PlatformError::UnsupportedOS(os.to_string())),
        }
    }

    pub fn from_str(os: &str) -> Result<Self, PlatformError> {
        match os.to_lowercase().as_str() {
            "linux" => Ok(OS::Linux(LinuxDistro::Generic)),
            "ubuntu" => Ok(OS::Linux(LinuxDistro::Ubuntu)),
            "darwin" => Ok(OS::Darwin),
            "windows" => Ok(OS::Windows),
            os => Err(PlatformError::UnsupportedOS(os.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub os: OS,
    pub arch: Architecture,
}

impl Platform {
    pub fn detect() -> Result<Self, PlatformError> {
        Ok(Self {
            os: OS::detect()?,
            arch: Architecture::detect()?,
        })
    }

    pub fn new(os: OS, arch: Architecture) -> Self {
        Self { os, arch }
    }

    pub fn get_release_package_name(&self, _version: &str) -> String {
        match &self.os {
            OS::Linux(distro) => match distro {
                LinuxDistro::Ubuntu => format!("ubuntu20.04_{}.tar.gz", self.arch),
                LinuxDistro::Generic => format!("manylinux2014_{}.tar.gz", self.arch),
            },
            OS::Darwin => format!("darwin_{}.tar.gz", self.arch),
            OS::Windows => format!("windows_{}.tar.gz", self.arch),
        }
    }
} 
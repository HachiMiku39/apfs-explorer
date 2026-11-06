//! Windows process architecture, independent of the machine running a build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsArch {
    X86,
    X64,
    Arm64,
}

impl WindowsArch {
    pub fn from_rust_arch(arch: &str) -> Option<Self> {
        match arch {
            "x86" => Some(Self::X86),
            "x86_64" => Some(Self::X64),
            "aarch64" => Some(Self::Arm64),
            _ => None,
        }
    }

    pub fn dll_name(self) -> &'static str {
        match self {
            Self::X86 => "winfsp-x86.dll",
            Self::X64 => "winfsp-x64.dll",
            Self::Arm64 => "winfsp-a64.dll",
        }
    }

    pub fn launchctl_name(self) -> &'static str {
        match self {
            Self::X86 => "launchctl-x86.exe",
            Self::X64 => "launchctl-x64.exe",
            Self::Arm64 => "launchctl-a64.exe",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WindowsArch;

    #[test]
    fn runtime_and_linker_select_the_same_target_architecture() {
        for (arch, dll, launcher) in [
            ("x86", "winfsp-x86.dll", "launchctl-x86.exe"),
            ("x86_64", "winfsp-x64.dll", "launchctl-x64.exe"),
            ("aarch64", "winfsp-a64.dll", "launchctl-a64.exe"),
        ] {
            let target = WindowsArch::from_rust_arch(arch).unwrap();
            assert_eq!(target.dll_name(), dll);
            assert_eq!(target.launchctl_name(), launcher);
        }
        assert!(WindowsArch::from_rust_arch("arm").is_none());
    }
}

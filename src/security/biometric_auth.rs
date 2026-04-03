#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiometricErrorCode {
    NotAvailable,
    NotEnrolled,
    LockedOut,
    UserCanceled,
    PermissionDenied,
    Timeout,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct BiometricAuthError {
    pub code: BiometricErrorCode,
    pub detail: String,
}

impl BiometricAuthError {
    pub fn i18n_key(&self) -> &'static str {
        match self.code {
            BiometricErrorCode::NotAvailable => "biometric.error.not_available",
            BiometricErrorCode::NotEnrolled => "biometric.error.not_enrolled",
            BiometricErrorCode::LockedOut => "biometric.error.locked_out",
            BiometricErrorCode::UserCanceled => "biometric.error.user_canceled",
            BiometricErrorCode::PermissionDenied => "biometric.error.permission_denied",
            BiometricErrorCode::Timeout => "biometric.error.timeout",
            BiometricErrorCode::Unknown => "biometric.error.unknown",
        }
    }
}

fn err(code: BiometricErrorCode, detail: impl Into<String>) -> BiometricAuthError {
    BiometricAuthError {
        code,
        detail: detail.into(),
    }
}

pub fn authenticate_user_presence(reason: &str) -> Result<(), BiometricAuthError> {
    let _ = reason;

    #[cfg(target_os = "macos")]
    {
        // Use LocalAuthentication natively via Swift runtime bridge.
        // This prompts Touch ID / Face ID where available.
        let swift = r#"
import Foundation
import LocalAuthentication

let reason = ProcessInfo.processInfo.environment["RUSTSSH_BIOMETRIC_REASON"] ?? "Authenticate"
let ctx = LAContext()
var evalError: NSError?

if !ctx.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &evalError) {
    if let e = evalError {
        if let code = LAError.Code(rawValue: e.code) {
            switch code {
            case .biometryNotAvailable:
                fputs("ERR_NOT_AVAILABLE\n", stderr)
            case .biometryNotEnrolled:
                fputs("ERR_NOT_ENROLLED\n", stderr)
            case .biometryLockout:
                fputs("ERR_LOCKED_OUT\n", stderr)
            case .passcodeNotSet:
                fputs("ERR_NOT_AVAILABLE\n", stderr)
            default:
                fputs("ERR_UNKNOWN:\(e.localizedDescription)\n", stderr)
            }
        } else {
            fputs("ERR_UNKNOWN:\(e.localizedDescription)\n", stderr)
        }
    } else {
        fputs("ERR_NOT_AVAILABLE\n", stderr)
    }
    exit(2)
}

let sem = DispatchSemaphore(value: 0)
var ok = false
var errLine: String?
ctx.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: reason) { success, error in
    ok = success
    if !success {
        if let e = error as NSError? {
            if let code = LAError.Code(rawValue: e.code) {
                switch code {
                case .userCancel, .appCancel, .systemCancel:
                    errLine = "ERR_USER_CANCELED"
                case .biometryLockout:
                    errLine = "ERR_LOCKED_OUT"
                case .biometryNotEnrolled:
                    errLine = "ERR_NOT_ENROLLED"
                case .biometryNotAvailable:
                    errLine = "ERR_NOT_AVAILABLE"
                case .authenticationFailed:
                    errLine = "ERR_PERMISSION_DENIED"
                default:
                    errLine = "ERR_UNKNOWN:\(e.localizedDescription)"
                }
            } else {
                errLine = "ERR_UNKNOWN:\(e.localizedDescription)"
            }
        } else {
            errLine = "ERR_UNKNOWN"
        }
    }
    sem.signal()
}
let waitRes = sem.wait(timeout: .now() + 45)
if waitRes == .timedOut {
    fputs("ERR_TIMEOUT\n", stderr)
    exit(3)
}
if let line = errLine {
    fputs("\(line)\n", stderr)
}
exit(ok ? 0 : 1)
"#;

        let output = std::process::Command::new("/usr/bin/swift")
            .arg("-e")
            .arg(swift)
            .env("RUSTSSH_BIOMETRIC_REASON", reason)
            .output()
            .map_err(|e| {
                err(
                    BiometricErrorCode::Unknown,
                    format!("调用系统认证失败: {}", e),
                )
            })?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("ERR_NOT_ENROLLED") {
            return Err(err(BiometricErrorCode::NotEnrolled, ""));
        }
        if stderr.contains("ERR_LOCKED_OUT") {
            return Err(err(BiometricErrorCode::LockedOut, ""));
        }
        if stderr.contains("ERR_USER_CANCELED") {
            return Err(err(BiometricErrorCode::UserCanceled, ""));
        }
        if stderr.contains("ERR_PERMISSION_DENIED") {
            return Err(err(BiometricErrorCode::PermissionDenied, ""));
        }
        if stderr.contains("ERR_TIMEOUT") {
            return Err(err(BiometricErrorCode::Timeout, ""));
        }
        if stderr.contains("ERR_NOT_AVAILABLE") {
            return Err(err(BiometricErrorCode::NotAvailable, ""));
        }
        return Err(err(BiometricErrorCode::Unknown, stderr.trim()));
    }

    #[cfg(target_os = "windows")]
    {
        // Windows Hello implementation slot: use UserConsentVerifier.
        // Works on supported Windows versions with Hello configured.
        let ps = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Security.Credentials.UI.UserConsentVerifier, Windows.Security.Credentials.UI, ContentType=WindowsRuntime]
$availability = [Windows.Security.Credentials.UI.UserConsentVerifier]::CheckAvailabilityAsync().AsTask().GetAwaiter().GetResult()
if ($availability -ne [Windows.Security.Credentials.UI.UserConsentVerifierAvailability]::Available) {
  Write-Error "Windows Hello unavailable: $availability"
  exit 2
}
$reason = $env:RUSTSSH_BIOMETRIC_REASON
$result = [Windows.Security.Credentials.UI.UserConsentVerifier]::RequestVerificationAsync($reason).AsTask().GetAwaiter().GetResult()
if ($result -eq [Windows.Security.Credentials.UI.UserConsentVerificationResult]::Verified) { exit 0 } else { exit 1 }
"#;
        let output = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(ps)
            .env("RUSTSSH_BIOMETRIC_REASON", reason)
            .output()
            .map_err(|e| {
                err(
                    BiometricErrorCode::Unknown,
                    format!("调用 Windows Hello 失败: {}", e),
                )
            })?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        if stderr.contains("notconfiguredforuser")
            || stderr.contains("device not")
            || stderr.contains("unavailable")
        {
            return Err(err(BiometricErrorCode::NotEnrolled, ""));
        }
        if stderr.contains("canceled") {
            return Err(err(BiometricErrorCode::UserCanceled, ""));
        }
        if stderr.contains("disabled") || stderr.contains("policy") {
            return Err(err(BiometricErrorCode::PermissionDenied, ""));
        }
        return Err(err(BiometricErrorCode::Unknown, stderr.trim()));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(err(
            BiometricErrorCode::NotAvailable,
            "platform biometric integration not implemented",
        ))
    }
}

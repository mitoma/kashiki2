fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    windows_bindgen::builder()
        .output("src/bindings/mod.rs")
        .filters([
            "Windows.Security.Credentials.KeyCredentialCreationOption",
            "Windows.Security.Credentials.KeyCredentialManager::RequestCreateAsync",
            "Windows.Security.Credentials.IKeyCredentialManagerStatics",
            "Windows.Security.Credentials.IKeyCredentialManagerStatics2",
            "Windows.Security.Credentials.IKeyCredentialManagerCreateWithWindowStatics",
            "Windows.Security.Credentials.IKeyCredentialCacheConfigurationFactory",
            "Windows.Security.Credentials.KeyCredentialRetrievalResult::{Status, Credential}",
            "Windows.Security.Credentials.UI.UserConsentVerifier::{RequestVerificationAsync, CheckAvailabilityAsync}",
            "Windows.Win32.IUserConsentVerifierInterop",
            "HWND",
            "FindWindowA",
            "SetForegroundWindow",
        ])
        .write();
}

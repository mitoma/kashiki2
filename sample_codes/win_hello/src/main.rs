/// Windows Hello のサンプルコード。bitwarden/clients が参考になる。
/// https://github.com/bitwarden/clients/blob/bcb2a976b094f57f1f7e1261e2692f12103d7b16/apps/desktop/desktop_native/src/biometric/windows.rs
fn main() {
    #[cfg(target_os = "windows")]
    windows::inner_main();
}

#[cfg(target_os = "windows")]
mod windows {
    use windows::{
        core::{factory, h, s, Result},
        Foundation::IAsyncOperation,
        Security::Credentials::{KeyCredentialCreationOption, KeyCredentialManager, UI::*},
        Win32::{
            Foundation::HWND,
            System::WinRT::IUserConsentVerifierInterop,
            UI::WindowsAndMessaging::{FindWindowA, SetForegroundWindow},
        },
    };
    use winit::{
        dpi::PhysicalSize,
        event::{ElementState, Event, KeyEvent, WindowEvent},
        event_loop::EventLoop,
        keyboard::Key,
        raw_window_handle::HasWindowHandle,
        window::WindowBuilder,
    };

    /// Windows Hello のサンプルコード。bitwarden/clients が参考になる。
    /// https://github.com/bitwarden/clients/blob/bcb2a976b094f57f1f7e1261e2692f12103d7b16/apps/desktop/desktop_native/src/biometric/windows.rs
    pub(crate) fn inner_main() {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(800, 600))
            .build(&event_loop)
            .unwrap();
        let window_handle = window.window_handle().unwrap();
        let raw_window_handle = window_handle.as_raw();
        let winit_hwnd = match raw_window_handle {
            winit::raw_window_handle::RawWindowHandle::Win32(handle) => handle.hwnd,
            _ => panic!("Not Windows"),
        };
        let hwnd = HWND(winit_hwnd.get());

        // Windows Hello 用のスレッドを作ってチャネルを持たせる
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            while rx.recv().is_ok() {
                call_hello(&hwnd).unwrap();
            }
        });

        event_loop
            .run(move |event, control_flow| match event {
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    logical_key: Key::Character(str),
                                    ..
                                },
                            ..
                        },
                    window_id,
                } => {
                    match str.as_str() {
                        "a" => {
                            tx.send(()).unwrap();
                            // あまりにも意味不明だが Credential Dialog Xaml Host のウィンドウを前面に出さないと
                            // Windows Hello の顔認証が失敗するため、ウインドウが出たであろうタイミングを待ってから最前面に移動させる。
                            let class_name = s!("Credential Dialog Xaml Host");
                            // 100 ms sleep する
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            unsafe {
                                let hello_hwnd = FindWindowA(class_name, None);
                                if hello_hwnd.0 != 0 {
                                    let _ = SetForegroundWindow(hello_hwnd);
                                }
                            }
                        }
                        "b" => {
                            call_hello(&HWND(0)).unwrap();
                        }
                        "s" => {
                            setup_first().unwrap();
                        }
                        "h" => {
                            println!("HWND: {:?}, WindowID: {:?}", hwnd, window_id);
                        }
                        _ => (),
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => control_flow.exit(),
                _ => (),
            })
            .unwrap();
    }

    fn setup_first() -> Result<()> {
        let key_result = KeyCredentialManager::RequestCreateAsync(
            h!("なんかいろいろ"),
            KeyCredentialCreationOption::ReplaceExisting,
        )?
        .get()?;
        let status = key_result.Status()?;
        println!("status: {:?}", status);
        let cred = key_result.Credential()?;
        println!("cred: {:?}", cred);
        //let blob = cred.RetrievePublicKeyWithDefaultBlobType()?;
        //println!("blob: {:?}", blob);
        Ok(())
    }

    fn call_hello(hwnd: &HWND) -> Result<()> {
        unsafe {
            println!("pre call factory");
            /*
            let operation =
                UserConsentVerifier::RequestVerificationAsync(h!("炊紙が利用者の認証を求めています"))?;
                 */
            let interop = factory::<UserConsentVerifier, IUserConsentVerifierInterop>()?;
            //let window = hwnd.clone(); // <== replace with your app's window handle
            println!("pre call RequestVerificationForWindowAsync");
            let operation: IAsyncOperation<UserConsentVerificationResult> =
                interop.RequestVerificationForWindowAsync(*hwnd, h!("Hello from Rust"))?;
            println!("post call RequestVerificationForWindowAsync");
            let result: UserConsentVerificationResult = operation.get()?;
            match result {
                UserConsentVerificationResult::Verified => println!("User verified"),
                UserConsentVerificationResult::DeviceNotPresent => println!("Device not present"),
                UserConsentVerificationResult::Canceled => println!("Canceled"),
                UserConsentVerificationResult::RetriesExhausted => println!("Retries exhausted"),
                UserConsentVerificationResult::DeviceBusy => println!("Device busy"),
                UserConsentVerificationResult::DisabledByPolicy => println!("Disabled by policy"),
                _ => (),
            }
            println!("{result:?}");
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn available() -> Result<bool> {
        let ucv_available = UserConsentVerifier::CheckAvailabilityAsync()?.get()?;

        match ucv_available {
            UserConsentVerifierAvailability::Available => Ok(true),
            UserConsentVerifierAvailability::DeviceBusy => Ok(true),
            _ => Ok(false),
        }
    }
}

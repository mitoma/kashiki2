/// Windows Hello のサンプルコード。bitwarden/clients が参考になる。
/// https://github.com/bitwarden/clients/blob/bcb2a976b094f57f1f7e1261e2692f12103d7b16/apps/desktop/desktop_native/src/biometric/windows.rs
fn main() {
    #[cfg(target_os = "windows")]
    windows::inner_main();
}

#[cfg(target_os = "windows")]
mod windows {

    use std::sync::mpsc::Sender;

    use pollster::FutureExt;
    use windows::{
        Security::Credentials::{KeyCredentialCreationOption, KeyCredentialManager, UI::*},
        Win32::{
            Foundation::HWND,
            System::WinRT::IUserConsentVerifierInterop,
            UI::WindowsAndMessaging::{FindWindowA, SetForegroundWindow},
        },
        core::{Result, factory, h, s},
    };
    use windows_future::IAsyncOperation;
    use winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::{ElementState, KeyEvent, WindowEvent},
        event_loop::EventLoop,
        keyboard::Key,
        raw_window_handle::{HasWindowHandle, Win32WindowHandle},
        window::{Window, WindowAttributes},
    };

    struct App {
        attributes: Option<AppAttributes>,
    }

    struct AppAttributes {
        window: Box<dyn Window>,
        tx: Sender<()>,
        handle: Win32WindowHandle,
    }

    impl ApplicationHandler for App {
        fn can_create_surfaces(&mut self, event_loop: &dyn winit::event_loop::ActiveEventLoop) {
            let window_attributes =
                WindowAttributes::default().with_surface_size(PhysicalSize::new(800, 600));
            self.attributes = match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let window_handle = window.window_handle().unwrap();
                    let raw_window_handle = window_handle.as_raw();
                    let handle = match raw_window_handle {
                        winit::raw_window_handle::RawWindowHandle::Win32(handle) => handle,
                        _ => panic!("Not Windows"),
                    };

                    // Windows Hello 用のスレッドを作ってチャネルを持たせる
                    let (tx, rx) = std::sync::mpsc::channel::<()>();
                    std::thread::spawn(move || {
                        let hwnd = to_hwnd(handle);
                        while rx.recv().is_ok() {
                            call_hello(&hwnd).block_on().unwrap();
                        }
                    });

                    Some(AppAttributes { window, tx, handle })
                }
                Err(err) => {
                    eprintln!("error creating window: {err}");
                    event_loop.exit();
                    return;
                }
            };
        }

        fn window_event(
            &mut self,
            event_loop: &dyn winit::event_loop::ActiveEventLoop,
            window_id: winit::window::WindowId,
            event: WindowEvent,
        ) {
            let Some(attrs) = &self.attributes else {
                return;
            };
            let self_window_id = attrs.window.id();

            match event {
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            logical_key: Key::Character(str),
                            ..
                        },
                    ..
                } => {
                    match str.as_str() {
                        "a" => {
                            attrs.tx.send(()).unwrap();
                            // あまりにも意味不明だが Credential Dialog Xaml Host のウィンドウを前面に出さないと
                            // Windows Hello の顔認証が失敗するため、ウインドウが出たであろうタイミングを待ってから最前面に移動させる。
                            let class_name = s!("Credential Dialog Xaml Host");
                            // 100 ms sleep する
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            unsafe {
                                if let Ok(hello_hwnd) = FindWindowA(class_name, None) {
                                    let _ = SetForegroundWindow(hello_hwnd);
                                }
                            }
                        }
                        "b" => {
                            call_hello(&HWND::default()).block_on().unwrap();
                        }
                        "s" => {
                            setup_first().block_on().unwrap();
                        }
                        "h" => {
                            println!("HWND: {:?}, WindowID: {:?}", attrs.handle, window_id);
                        }
                        _ => (),
                    }
                }
                WindowEvent::CloseRequested if window_id == self_window_id => event_loop.exit(),
                _ => return,
            }
        }
    }

    fn to_hwnd(handle: Win32WindowHandle) -> HWND {
        HWND(handle.hwnd.get() as *mut std::ffi::c_void)
    }

    /// Windows Hello のサンプルコード。bitwarden/clients が参考になる。
    /// https://github.com/bitwarden/clients/blob/bcb2a976b094f57f1f7e1261e2692f12103d7b16/apps/desktop/desktop_native/src/biometric/windows.rs
    pub(crate) fn inner_main() {
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(App { attributes: None }).unwrap();
    }

    async fn setup_first() -> Result<()> {
        let key_result = KeyCredentialManager::RequestCreateAsync(
            h!("なんかいろいろ"),
            KeyCredentialCreationOption::ReplaceExisting,
        )?
        .await?;
        //.get()?;
        let status = key_result.Status()?;
        println!("status: {:?}", status);
        let cred = key_result.Credential();
        println!("cred: {:?}", cred);
        //let blob = cred.RetrievePublicKeyWithDefaultBlobType()?;
        //println!("blob: {:?}", blob);
        Ok(())
    }

    async fn call_hello(hwnd: &HWND) -> Result<()> {
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
            let result: UserConsentVerificationResult = operation.await?;
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
    async fn available() -> Result<bool> {
        let ucv_available = UserConsentVerifier::CheckAvailabilityAsync()?.await?;

        match ucv_available {
            UserConsentVerifierAvailability::Available => Ok(true),
            UserConsentVerifierAvailability::DeviceBusy => Ok(true),
            _ => Ok(false),
        }
    }
}

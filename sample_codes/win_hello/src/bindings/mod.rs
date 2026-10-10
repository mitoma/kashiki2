pub mod Windows {
    pub mod Security {
        pub mod Credentials {
            windows_core::imp::define_interface!(
                AttestationChallengeHandler,
                AttestationChallengeHandler_Vtbl,
                0xf6ae35b0_d805_587d_944f_a09bd032acf5
            );
            impl windows_core::RuntimeType for AttestationChallengeHandler {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
            }
            impl AttestationChallengeHandler {
                pub fn new<
                    F: Fn(
                            windows_core::Ref<super::super::Storage::Streams::IBuffer>,
                        )
                            -> windows_core::Result<super::super::Storage::Streams::IBuffer>
                        + Send
                        + 'static,
                >(
                    invoke: F,
                ) -> Self {
                    let com = windows_core::imp::DelegateBox::<Self, F>::new(
                        &AttestationChallengeHandlerBox::<F>::VTABLE,
                        invoke,
                    );
                    unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
                }
                pub fn Invoke<P0>(
                    &self,
                    challenge: P0,
                ) -> windows_core::Result<super::super::Storage::Streams::IBuffer>
                where
                    P0: windows_core::Param<super::super::Storage::Streams::IBuffer>,
                {
                    unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(self).Invoke)(
                            windows_core::Interface::as_raw(self),
                            challenge.param().abi(),
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    }
                }
            }
            #[repr(C)]
            pub struct AttestationChallengeHandler_Vtbl {
                base__: windows_core::IUnknown_Vtbl,
                Invoke: unsafe extern "system" fn(
                    this: *mut core::ffi::c_void,
                    challenge: *mut core::ffi::c_void,
                    result__: *mut *mut core::ffi::c_void,
                ) -> windows_core::HRESULT,
            }
            struct AttestationChallengeHandlerBox<
                F: Fn(
                        windows_core::Ref<super::super::Storage::Streams::IBuffer>,
                    )
                        -> windows_core::Result<super::super::Storage::Streams::IBuffer>
                    + Send
                    + 'static,
            >(core::marker::PhantomData<(fn() -> F,)>);
            impl<
                F: Fn(
                        windows_core::Ref<super::super::Storage::Streams::IBuffer>,
                    )
                        -> windows_core::Result<super::super::Storage::Streams::IBuffer>
                    + Send
                    + 'static,
            > AttestationChallengeHandlerBox<F>
            {
                const VTABLE: AttestationChallengeHandler_Vtbl =
                    AttestationChallengeHandler_Vtbl {
                        base__:
                            windows_core::IUnknown_Vtbl {
                                QueryInterface: windows_core::imp::DelegateBox::<
                                    AttestationChallengeHandler,
                                    F,
                                >::QueryInterface,
                                AddRef: windows_core::imp::DelegateBox::<
                                    AttestationChallengeHandler,
                                    F,
                                >::AddRef,
                                Release: windows_core::imp::DelegateBox::<
                                    AttestationChallengeHandler,
                                    F,
                                >::Release,
                            },
                        Invoke: Self::Invoke,
                    };
                unsafe extern "system" fn Invoke(
                    this: *mut core::ffi::c_void,
                    challenge: *mut core::ffi::c_void,
                    result__: *mut *mut core::ffi::c_void,
                ) -> windows_core::HRESULT {
                    unsafe {
                        let this = &mut *(this as *mut *mut core::ffi::c_void
                            as *mut windows_core::imp::DelegateBox<AttestationChallengeHandler, F>);
                        match (this.invoke)(core::mem::transmute_copy(&challenge)) {
                            Ok(ok__) => {
                                result__.write(core::mem::transmute_copy(&ok__));
                                core::mem::forget(ok__);
                                windows_core::HRESULT(0)
                            }
                            Err(err) => err.into(),
                        }
                    }
                }
            }
            #[repr(transparent)]
            #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
            pub struct ChallengeResponseKind(pub i32);
            impl ChallengeResponseKind {
                pub const VirtualizationBasedSecurityEnclave: Self = Self(0);
            }
            impl windows_core::imp::TypeKind for ChallengeResponseKind {
                type TypeKind = windows_core::imp::CopyType;
            }
            impl windows_core::RuntimeType for ChallengeResponseKind {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"enum(Windows.Security.Credentials.ChallengeResponseKind;i4)",
                    );
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.ChallengeResponseKind",
                    );
            }
            windows_core::imp::define_interface!(
                IKeyCredential,
                IKeyCredential_Vtbl,
                0x9585ef8d_457b_4847_b11a_fa960bbdb138
            );
            impl windows_core::RuntimeType for IKeyCredential {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.IKeyCredential",
                    );
            }
            #[repr(C)]
            pub struct IKeyCredential_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
            }
            windows_core::imp::define_interface!(
                IKeyCredentialCacheConfiguration,
                IKeyCredentialCacheConfiguration_Vtbl,
                0x438bd21a_61ff_5468_95a6_b1d5216e458d
            );
            impl windows_core::RuntimeType for IKeyCredentialCacheConfiguration {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.IKeyCredentialCacheConfiguration",
                    );
            }
            #[repr(C)]
            pub struct IKeyCredentialCacheConfiguration_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
            }
            windows_core::imp::define_interface!(
                IKeyCredentialCacheConfigurationFactory,
                IKeyCredentialCacheConfigurationFactory_Vtbl,
                0x9948c31b_c827_5b58_9442_40acd8ab1e7d
            );
            impl windows_core::RuntimeType for IKeyCredentialCacheConfigurationFactory {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.IKeyCredentialCacheConfigurationFactory",
                    );
            }
            windows_core::imp::interface_hierarchy!(
                IKeyCredentialCacheConfigurationFactory,
                windows_core::IUnknown,
                windows_core::IInspectable
            );
            impl IKeyCredentialCacheConfigurationFactory {
                pub fn CreateInstance(
                    &self,
                    cacheoption: KeyCredentialCacheOption,
                    timeout: windows_time::TimeSpan,
                    usagecount: u32,
                ) -> windows_core::Result<KeyCredentialCacheConfiguration> {
                    unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(self).CreateInstance)(
                            windows_core::Interface::as_raw(self),
                            cacheoption,
                            timeout,
                            usagecount,
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    }
                }
            }
            impl windows_core::RuntimeName for IKeyCredentialCacheConfigurationFactory {
                const NAME: &'static str =
                    "Windows.Security.Credentials.IKeyCredentialCacheConfigurationFactory";
            }
            pub trait IKeyCredentialCacheConfigurationFactory_Impl:
                windows_core::IUnknownImpl
            {
                fn CreateInstance(
                    &self,
                    cacheOption: KeyCredentialCacheOption,
                    timeout: &windows_time::TimeSpan,
                    usageCount: u32,
                ) -> windows_core::Result<KeyCredentialCacheConfiguration>;
            }
            impl IKeyCredentialCacheConfigurationFactory_Vtbl {
                pub const fn new<
                    Identity: IKeyCredentialCacheConfigurationFactory_Impl,
                    const OFFSET: isize,
                >() -> Self {
                    unsafe extern "system" fn CreateInstance<
                        Identity: IKeyCredentialCacheConfigurationFactory_Impl,
                        const OFFSET: isize,
                    >(
                        this: *mut core::ffi::c_void,
                        cacheoption: KeyCredentialCacheOption,
                        timeout: windows_time::TimeSpan,
                        usagecount: u32,
                        result__: *mut *mut core::ffi::c_void,
                    ) -> windows_core::HRESULT {
                        unsafe {
                            let this: &Identity =
                                &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                            match IKeyCredentialCacheConfigurationFactory_Impl::CreateInstance(
                                this,
                                cacheoption,
                                core::mem::transmute(&timeout),
                                usagecount,
                            ) {
                                Ok(ok__) => {
                                    result__.write(core::mem::transmute_copy(&ok__));
                                    core::mem::forget(ok__);
                                    windows_core::HRESULT(0)
                                }
                                Err(err) => err.into(),
                            }
                        }
                    }
                    Self {
                        base__: windows_core::IInspectable_Vtbl::new::<
                            Identity,
                            IKeyCredentialCacheConfigurationFactory,
                            OFFSET,
                        >(),
                        CreateInstance: CreateInstance::<Identity, OFFSET>,
                    }
                }
                pub fn matches(iid: &windows_core::GUID) -> bool {
                    iid == & < IKeyCredentialCacheConfigurationFactory as windows_core::Interface >::IID
                }
            }
            #[repr(C)]
            pub struct IKeyCredentialCacheConfigurationFactory_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
                pub CreateInstance: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    KeyCredentialCacheOption,
                    windows_time::TimeSpan,
                    u32,
                    *mut *mut core::ffi::c_void,
                )
                    -> windows_core::HRESULT,
            }
            windows_core::imp::define_interface!(
                IKeyCredentialManagerCreateWithWindowStatics,
                IKeyCredentialManagerCreateWithWindowStatics_Vtbl,
                0x30b1b9c9_61ef_43e8_88ac_cc433b38d1a6
            );
            impl windows_core::RuntimeType for IKeyCredentialManagerCreateWithWindowStatics {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME : windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice (b"Windows.Security.Credentials.IKeyCredentialManagerCreateWithWindowStatics") ;
            }
            #[repr(C)]
            pub struct IKeyCredentialManagerCreateWithWindowStatics_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
                pub RequestCreateForWindowAsync: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    super::super::UI::WindowId,
                    *mut core::ffi::c_void,
                    KeyCredentialCreationOption,
                    *mut *mut core::ffi::c_void,
                )
                    -> windows_core::HRESULT,
            }
            windows_core::imp::define_interface!(
                IKeyCredentialManagerStatics,
                IKeyCredentialManagerStatics_Vtbl,
                0x6aac468b_0ef1_4ce0_8290_4106da6a63b5
            );
            impl windows_core::RuntimeType for IKeyCredentialManagerStatics {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.IKeyCredentialManagerStatics",
                    );
            }
            #[repr(C)]
            pub struct IKeyCredentialManagerStatics_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
                IsSupportedAsync: usize,
                RenewAttestationAsync: usize,
                pub RequestCreateAsync: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    *mut core::ffi::c_void,
                    KeyCredentialCreationOption,
                    *mut *mut core::ffi::c_void,
                )
                    -> windows_core::HRESULT,
            }
            windows_core::imp::define_interface!(
                IKeyCredentialManagerStatics2,
                IKeyCredentialManagerStatics2_Vtbl,
                0x6439895d_68c5_521b_9dc4_7c199794f0d8
            );
            impl windows_core::RuntimeType for IKeyCredentialManagerStatics2 {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.IKeyCredentialManagerStatics2",
                    );
            }
            #[repr(C)]
            pub struct IKeyCredentialManagerStatics2_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
                pub RequestCreateAsync: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    *mut core::ffi::c_void,
                    KeyCredentialCreationOption,
                    *mut core::ffi::c_void,
                    *mut core::ffi::c_void,
                    *mut core::ffi::c_void,
                    super::super::UI::WindowId,
                    ChallengeResponseKind,
                    *mut core::ffi::c_void,
                    *mut *mut core::ffi::c_void,
                )
                    -> windows_core::HRESULT,
                pub OpenAsync: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    *mut core::ffi::c_void,
                    ChallengeResponseKind,
                    *mut core::ffi::c_void,
                    *mut *mut core::ffi::c_void,
                ) -> windows_core::HRESULT,
                pub GetSecureId: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    *mut *mut core::ffi::c_void,
                )
                    -> windows_core::HRESULT,
            }
            windows_core::imp::define_interface!(
                IKeyCredentialRetrievalResult,
                IKeyCredentialRetrievalResult_Vtbl,
                0x58cd7703_8d87_4249_9b58_f6598cc9644e
            );
            impl windows_core::RuntimeType for IKeyCredentialRetrievalResult {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.IKeyCredentialRetrievalResult",
                    );
            }
            #[repr(C)]
            pub struct IKeyCredentialRetrievalResult_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
                pub Credential: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    *mut *mut core::ffi::c_void,
                ) -> windows_core::HRESULT,
                pub Status: unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    *mut KeyCredentialStatus,
                ) -> windows_core::HRESULT,
            }
            #[repr(transparent)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub struct KeyCredential(windows_core::IUnknown);
            windows_core::imp::interface_hierarchy!(
                KeyCredential,
                windows_core::IUnknown,
                windows_core::IInspectable
            );
            impl windows_core::RuntimeType for KeyCredential {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_class::<Self, IKeyCredential>();
            }
            unsafe impl windows_core::Interface for KeyCredential {
                type Vtable = <IKeyCredential as windows_core::Interface>::Vtable;
                const IID: windows_core::GUID = <IKeyCredential as windows_core::Interface>::IID;
            }
            impl windows_core::RuntimeName for KeyCredential {
                const NAME: &'static str = "Windows.Security.Credentials.KeyCredential";
            }
            unsafe impl Send for KeyCredential {}
            unsafe impl Sync for KeyCredential {}
            #[repr(transparent)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub struct KeyCredentialCacheConfiguration(windows_core::IUnknown);
            windows_core::imp::interface_hierarchy!(
                KeyCredentialCacheConfiguration,
                windows_core::IUnknown,
                windows_core::IInspectable
            );
            impl KeyCredentialCacheConfiguration {
                pub fn CreateInstance(
                    cacheoption: KeyCredentialCacheOption,
                    timeout: windows_time::TimeSpan,
                    usagecount: u32,
                ) -> windows_core::Result<Self> {
                    Self::IKeyCredentialCacheConfigurationFactory(|this| unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(this).CreateInstance)(
                            windows_core::Interface::as_raw(this),
                            cacheoption,
                            timeout,
                            usagecount,
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    })
                }
                fn IKeyCredentialCacheConfigurationFactory<
                    R,
                    F: FnOnce(&IKeyCredentialCacheConfigurationFactory) -> windows_core::Result<R>,
                >(
                    callback: F,
                ) -> windows_core::Result<R> {
                    static SHARED: windows_core::imp::FactoryCache<
                        KeyCredentialCacheConfiguration,
                        IKeyCredentialCacheConfigurationFactory,
                    > = windows_core::imp::FactoryCache::new();
                    SHARED.call(callback)
                }
            }
            impl windows_core::RuntimeType for KeyCredentialCacheConfiguration {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_class::<
                        Self,
                        IKeyCredentialCacheConfiguration,
                    >();
            }
            unsafe impl windows_core::Interface for KeyCredentialCacheConfiguration {
                type Vtable = <IKeyCredentialCacheConfiguration as windows_core::Interface>::Vtable;
                const IID: windows_core::GUID =
                    <IKeyCredentialCacheConfiguration as windows_core::Interface>::IID;
            }
            impl windows_core::RuntimeName for KeyCredentialCacheConfiguration {
                const NAME: &'static str =
                    "Windows.Security.Credentials.KeyCredentialCacheConfiguration";
            }
            unsafe impl Send for KeyCredentialCacheConfiguration {}
            unsafe impl Sync for KeyCredentialCacheConfiguration {}
            #[repr(transparent)]
            #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
            pub struct KeyCredentialCacheOption(pub i32);
            impl KeyCredentialCacheOption {
                pub const NoCache: Self = Self(0);
                pub const CacheWhenUnlocked: Self = Self(1);
            }
            impl windows_core::imp::TypeKind for KeyCredentialCacheOption {
                type TypeKind = windows_core::imp::CopyType;
            }
            impl windows_core::RuntimeType for KeyCredentialCacheOption {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"enum(Windows.Security.Credentials.KeyCredentialCacheOption;i4)",
                    );
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.KeyCredentialCacheOption",
                    );
            }
            #[repr(transparent)]
            #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
            pub struct KeyCredentialCreationOption(pub i32);
            impl KeyCredentialCreationOption {
                pub const ReplaceExisting: Self = Self(0);
                pub const FailIfExists: Self = Self(1);
            }
            impl windows_core::imp::TypeKind for KeyCredentialCreationOption {
                type TypeKind = windows_core::imp::CopyType;
            }
            impl windows_core::RuntimeType for KeyCredentialCreationOption {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"enum(Windows.Security.Credentials.KeyCredentialCreationOption;i4)",
                    );
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.KeyCredentialCreationOption",
                    );
            }
            pub struct KeyCredentialManager;
            impl KeyCredentialManager {
                pub fn RequestCreateForWindowAsync(
                    window: super::super::UI::WindowId,
                    name: &windows_core::HSTRING,
                    option: KeyCredentialCreationOption,
                ) -> windows_core::Result<
                    windows_future::IAsyncOperation<KeyCredentialRetrievalResult>,
                > {
                    Self::IKeyCredentialManagerCreateWithWindowStatics(|this| unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(this).RequestCreateForWindowAsync)(
                            windows_core::Interface::as_raw(this),
                            window,
                            core::mem::transmute_copy(name),
                            option,
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    })
                }
                pub fn RequestCreateAsync(
                    name: &windows_core::HSTRING,
                    option: KeyCredentialCreationOption,
                ) -> windows_core::Result<
                    windows_future::IAsyncOperation<KeyCredentialRetrievalResult>,
                > {
                    Self::IKeyCredentialManagerStatics(|this| unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(this).RequestCreateAsync)(
                            windows_core::Interface::as_raw(this),
                            core::mem::transmute_copy(name),
                            option,
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    })
                }
                pub fn RequestCreateAsync2<P4, P7>(
                    name: &windows_core::HSTRING,
                    option: KeyCredentialCreationOption,
                    algorithm: &windows_core::HSTRING,
                    message: &windows_core::HSTRING,
                    cacheconfiguration: P4,
                    windowid: super::super::UI::WindowId,
                    callbacktype: ChallengeResponseKind,
                    attestationcallback: P7,
                ) -> windows_core::Result<
                    windows_future::IAsyncOperation<KeyCredentialRetrievalResult>,
                >
                where
                    P4: windows_core::Param<KeyCredentialCacheConfiguration>,
                    P7: windows_core::Param<AttestationChallengeHandler>,
                {
                    Self::IKeyCredentialManagerStatics2(|this| unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(this).RequestCreateAsync)(
                            windows_core::Interface::as_raw(this),
                            core::mem::transmute_copy(name),
                            option,
                            core::mem::transmute_copy(algorithm),
                            core::mem::transmute_copy(message),
                            cacheconfiguration.param().abi(),
                            windowid,
                            callbacktype,
                            attestationcallback.param().abi(),
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    })
                }
                pub fn OpenAsync<P2>(
                    name: &windows_core::HSTRING,
                    callbacktype: ChallengeResponseKind,
                    attestationcallback: P2,
                ) -> windows_core::Result<
                    windows_future::IAsyncOperation<KeyCredentialRetrievalResult>,
                >
                where
                    P2: windows_core::Param<AttestationChallengeHandler>,
                {
                    Self::IKeyCredentialManagerStatics2(|this| unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(this).OpenAsync)(
                            windows_core::Interface::as_raw(this),
                            core::mem::transmute_copy(name),
                            callbacktype,
                            attestationcallback.param().abi(),
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    })
                }
                pub fn GetSecureId() -> windows_core::Result<super::super::Storage::Streams::IBuffer>
                {
                    Self::IKeyCredentialManagerStatics2(|this| unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(this).GetSecureId)(
                            windows_core::Interface::as_raw(this),
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    })
                }
                fn IKeyCredentialManagerCreateWithWindowStatics<
                    R,
                    F: FnOnce(
                        &IKeyCredentialManagerCreateWithWindowStatics,
                    ) -> windows_core::Result<R>,
                >(
                    callback: F,
                ) -> windows_core::Result<R> {
                    static SHARED: windows_core::imp::FactoryCache<
                        KeyCredentialManager,
                        IKeyCredentialManagerCreateWithWindowStatics,
                    > = windows_core::imp::FactoryCache::new();
                    SHARED.call(callback)
                }
                fn IKeyCredentialManagerStatics<
                    R,
                    F: FnOnce(&IKeyCredentialManagerStatics) -> windows_core::Result<R>,
                >(
                    callback: F,
                ) -> windows_core::Result<R> {
                    static SHARED: windows_core::imp::FactoryCache<
                        KeyCredentialManager,
                        IKeyCredentialManagerStatics,
                    > = windows_core::imp::FactoryCache::new();
                    SHARED.call(callback)
                }
                fn IKeyCredentialManagerStatics2<
                    R,
                    F: FnOnce(&IKeyCredentialManagerStatics2) -> windows_core::Result<R>,
                >(
                    callback: F,
                ) -> windows_core::Result<R> {
                    static SHARED: windows_core::imp::FactoryCache<
                        KeyCredentialManager,
                        IKeyCredentialManagerStatics2,
                    > = windows_core::imp::FactoryCache::new();
                    SHARED.call(callback)
                }
            }
            impl windows_core::RuntimeName for KeyCredentialManager {
                const NAME: &'static str = "Windows.Security.Credentials.KeyCredentialManager";
            }
            #[repr(transparent)]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub struct KeyCredentialRetrievalResult(windows_core::IUnknown);
            windows_core::imp::interface_hierarchy!(
                KeyCredentialRetrievalResult,
                windows_core::IUnknown,
                windows_core::IInspectable
            );
            impl KeyCredentialRetrievalResult {
                pub fn Credential(&self) -> windows_core::Result<KeyCredential> {
                    unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(self).Credential)(
                            windows_core::Interface::as_raw(self),
                            &mut result__,
                        )
                        .and_then(|| windows_core::imp::Type::from_abi(result__))
                    }
                }
                pub fn Status(&self) -> windows_core::Result<KeyCredentialStatus> {
                    unsafe {
                        let mut result__ = core::mem::zeroed();
                        (windows_core::Interface::vtable(self).Status)(
                            windows_core::Interface::as_raw(self),
                            &mut result__,
                        )
                        .map(|| result__)
                    }
                }
            }
            impl windows_core::RuntimeType for KeyCredentialRetrievalResult {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_class::<Self, IKeyCredentialRetrievalResult>(
                    );
            }
            unsafe impl windows_core::Interface for KeyCredentialRetrievalResult {
                type Vtable = <IKeyCredentialRetrievalResult as windows_core::Interface>::Vtable;
                const IID: windows_core::GUID =
                    <IKeyCredentialRetrievalResult as windows_core::Interface>::IID;
            }
            impl windows_core::RuntimeName for KeyCredentialRetrievalResult {
                const NAME: &'static str =
                    "Windows.Security.Credentials.KeyCredentialRetrievalResult";
            }
            unsafe impl Send for KeyCredentialRetrievalResult {}
            unsafe impl Sync for KeyCredentialRetrievalResult {}
            #[repr(transparent)]
            #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
            pub struct KeyCredentialStatus(pub i32);
            impl KeyCredentialStatus {
                pub const Success: Self = Self(0);
                pub const UnknownError: Self = Self(1);
                pub const NotFound: Self = Self(2);
                pub const UserCanceled: Self = Self(3);
                pub const UserPrefersPassword: Self = Self(4);
                pub const CredentialAlreadyExists: Self = Self(5);
                pub const SecurityDeviceLocked: Self = Self(6);
                pub const AlgorithmNotSupported: Self = Self(7);
            }
            impl windows_core::imp::TypeKind for KeyCredentialStatus {
                type TypeKind = windows_core::imp::CopyType;
            }
            impl windows_core::RuntimeType for KeyCredentialStatus {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"enum(Windows.Security.Credentials.KeyCredentialStatus;i4)",
                    );
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(
                        b"Windows.Security.Credentials.KeyCredentialStatus",
                    );
            }
            pub mod UI {
                windows_core::imp::define_interface!(
                    IUserConsentVerifierStatics,
                    IUserConsentVerifierStatics_Vtbl,
                    0xaf4f3f91_564c_4ddc_b8b5_973447627c65
                );
                impl windows_core::RuntimeType for IUserConsentVerifierStatics {
                    const SIGNATURE: windows_core::imp::ConstBuffer =
                        windows_core::imp::ConstBuffer::for_interface::<Self>();
                    const NAME: windows_core::imp::ConstBuffer =
                        windows_core::imp::ConstBuffer::from_slice(
                            b"Windows.Security.Credentials.UI.IUserConsentVerifierStatics",
                        );
                }
                #[repr(C)]
                pub struct IUserConsentVerifierStatics_Vtbl {
                    pub base__: windows_core::IInspectable_Vtbl,
                    pub CheckAvailabilityAsync: unsafe extern "system" fn(
                        *mut core::ffi::c_void,
                        *mut *mut core::ffi::c_void,
                    )
                        -> windows_core::HRESULT,
                    pub RequestVerificationAsync:
                        unsafe extern "system" fn(
                            *mut core::ffi::c_void,
                            *mut core::ffi::c_void,
                            *mut *mut core::ffi::c_void,
                        ) -> windows_core::HRESULT,
                }
                #[repr(transparent)]
                #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
                pub struct UserConsentVerificationResult(pub i32);
                impl UserConsentVerificationResult {
                    pub const Verified: Self = Self(0);
                    pub const DeviceNotPresent: Self = Self(1);
                    pub const NotConfiguredForUser: Self = Self(2);
                    pub const DisabledByPolicy: Self = Self(3);
                    pub const DeviceBusy: Self = Self(4);
                    pub const RetriesExhausted: Self = Self(5);
                    pub const Canceled: Self = Self(6);
                }
                impl windows_core::imp::TypeKind for UserConsentVerificationResult {
                    type TypeKind = windows_core::imp::CopyType;
                }
                impl windows_core::RuntimeType for UserConsentVerificationResult {
                    const SIGNATURE : windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice (b"enum(Windows.Security.Credentials.UI.UserConsentVerificationResult;i4)") ;
                    const NAME: windows_core::imp::ConstBuffer =
                        windows_core::imp::ConstBuffer::from_slice(
                            b"Windows.Security.Credentials.UI.UserConsentVerificationResult",
                        );
                }
                pub struct UserConsentVerifier;
                impl UserConsentVerifier {
                    pub fn CheckAvailabilityAsync() -> windows_core::Result<
                        windows_future::IAsyncOperation<UserConsentVerifierAvailability>,
                    > {
                        Self::IUserConsentVerifierStatics(|this| unsafe {
                            let mut result__ = core::mem::zeroed();
                            (windows_core::Interface::vtable(this).CheckAvailabilityAsync)(
                                windows_core::Interface::as_raw(this),
                                &mut result__,
                            )
                            .and_then(|| windows_core::imp::Type::from_abi(result__))
                        })
                    }
                    pub fn RequestVerificationAsync(
                        message: &windows_core::HSTRING,
                    ) -> windows_core::Result<
                        windows_future::IAsyncOperation<UserConsentVerificationResult>,
                    > {
                        Self::IUserConsentVerifierStatics(|this| unsafe {
                            let mut result__ = core::mem::zeroed();
                            (windows_core::Interface::vtable(this).RequestVerificationAsync)(
                                windows_core::Interface::as_raw(this),
                                core::mem::transmute_copy(message),
                                &mut result__,
                            )
                            .and_then(|| windows_core::imp::Type::from_abi(result__))
                        })
                    }
                    fn IUserConsentVerifierStatics<
                        R,
                        F: FnOnce(&IUserConsentVerifierStatics) -> windows_core::Result<R>,
                    >(
                        callback: F,
                    ) -> windows_core::Result<R> {
                        static SHARED: windows_core::imp::FactoryCache<
                            UserConsentVerifier,
                            IUserConsentVerifierStatics,
                        > = windows_core::imp::FactoryCache::new();
                        SHARED.call(callback)
                    }
                }
                impl windows_core::RuntimeName for UserConsentVerifier {
                    const NAME: &'static str =
                        "Windows.Security.Credentials.UI.UserConsentVerifier";
                }
                #[repr(transparent)]
                #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
                pub struct UserConsentVerifierAvailability(pub i32);
                impl UserConsentVerifierAvailability {
                    pub const Available: Self = Self(0);
                    pub const DeviceNotPresent: Self = Self(1);
                    pub const NotConfiguredForUser: Self = Self(2);
                    pub const DisabledByPolicy: Self = Self(3);
                    pub const DeviceBusy: Self = Self(4);
                }
                impl windows_core::imp::TypeKind for UserConsentVerifierAvailability {
                    type TypeKind = windows_core::imp::CopyType;
                }
                impl windows_core::RuntimeType for UserConsentVerifierAvailability {
                    const SIGNATURE : windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice (b"enum(Windows.Security.Credentials.UI.UserConsentVerifierAvailability;i4)") ;
                    const NAME: windows_core::imp::ConstBuffer =
                        windows_core::imp::ConstBuffer::from_slice(
                            b"Windows.Security.Credentials.UI.UserConsentVerifierAvailability",
                        );
                }
            }
        }
    }
    pub mod Storage {
        pub mod Streams {
            windows_core::imp::define_interface!(
                IBuffer,
                IBuffer_Vtbl,
                0x905a0fe0_bc53_11df_8c49_001e4fc686da
            );
            impl windows_core::RuntimeType for IBuffer {
                const SIGNATURE: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::for_interface::<Self>();
                const NAME: windows_core::imp::ConstBuffer =
                    windows_core::imp::ConstBuffer::from_slice(b"Windows.Storage.Streams.IBuffer");
            }
            windows_core::imp::interface_hierarchy!(
                IBuffer,
                windows_core::IUnknown,
                windows_core::IInspectable
            );
            impl windows_core::RuntimeName for IBuffer {
                const NAME: &'static str = "Windows.Storage.Streams.IBuffer";
            }
            #[repr(C)]
            pub struct IBuffer_Vtbl {
                pub base__: windows_core::IInspectable_Vtbl,
            }
        }
    }
    pub mod UI {
        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub struct WindowId {
            pub Value: u64,
        }
        impl windows_core::imp::TypeKind for WindowId {
            type TypeKind = windows_core::imp::CopyType;
        }
        impl windows_core::RuntimeType for WindowId {
            const SIGNATURE: windows_core::imp::ConstBuffer =
                windows_core::imp::ConstBuffer::from_slice(b"struct(Windows.UI.WindowId;u8)");
            const NAME: windows_core::imp::ConstBuffer =
                windows_core::imp::ConstBuffer::from_slice(b"Windows.UI.WindowId");
        }
    }
    pub mod Win32 {
        #[inline]
        pub unsafe fn FindWindowA<P0, P1>(lpclassname: P0, lpwindowname: P1) -> HWND
        where
            P0: windows_core::Param<windows_core::PCSTR>,
            P1: windows_core::Param<windows_core::PCSTR>,
        {
            windows_core::link!("user32.dll" "system" fn FindWindowA(lpclassname : windows_core::PCSTR, lpwindowname : windows_core::PCSTR) -> HWND);
            unsafe { FindWindowA(lpclassname.param().abi(), lpwindowname.param().abi()) }
        }
        #[inline]
        pub unsafe fn SetForegroundWindow(hwnd: HWND) -> windows_core::BOOL {
            windows_core::link!("user32.dll" "system" fn SetForegroundWindow(hwnd : HWND) -> windows_core::BOOL);
            unsafe { SetForegroundWindow(hwnd) }
        }
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
        pub struct HWND(pub *mut core::ffi::c_void);
        windows_core::imp::define_interface!(
            IUserConsentVerifierInterop,
            IUserConsentVerifierInterop_Vtbl,
            0x39e050c3_4e74_441a_8dc0_b81104df949c
        );
        windows_core::imp::interface_hierarchy!(
            IUserConsentVerifierInterop,
            windows_core::IUnknown,
            windows_core::IInspectable
        );
        impl IUserConsentVerifierInterop {
            pub unsafe fn RequestVerificationForWindowAsync<T>(
                &self,
                appwindow: HWND,
                message: &windows_core::HSTRING,
            ) -> windows_core::Result<T>
            where
                T: windows_core::Interface,
            {
                let mut result__ = core::ptr::null_mut();
                unsafe {
                    (windows_core::Interface::vtable(self).RequestVerificationForWindowAsync)(
                        windows_core::Interface::as_raw(self),
                        appwindow,
                        core::mem::transmute_copy(message),
                        &T::IID,
                        &mut result__,
                    )
                    .and_then(|| windows_core::imp::Type::from_abi(result__))
                }
            }
        }
        #[repr(C)]
        pub struct IUserConsentVerifierInterop_Vtbl {
            pub base__: windows_core::IInspectable_Vtbl,
            pub RequestVerificationForWindowAsync:
                unsafe extern "system" fn(
                    *mut core::ffi::c_void,
                    HWND,
                    *mut core::ffi::c_void,
                    *const windows_core::GUID,
                    *mut *mut core::ffi::c_void,
                ) -> windows_core::HRESULT,
        }
        pub trait IUserConsentVerifierInterop_Impl: windows_core::IUnknownImpl {
            fn RequestVerificationForWindowAsync(
                &self,
                appwindow: HWND,
                message: &windows_core::HSTRING,
                riid: *const windows_core::GUID,
                asyncoperation: *mut *mut core::ffi::c_void,
            ) -> windows_core::Result<()>;
        }
        impl IUserConsentVerifierInterop_Vtbl {
            pub const fn new<Identity: IUserConsentVerifierInterop_Impl, const OFFSET: isize>()
            -> Self {
                unsafe extern "system" fn RequestVerificationForWindowAsync<
                    Identity: IUserConsentVerifierInterop_Impl,
                    const OFFSET: isize,
                >(
                    this: *mut core::ffi::c_void,
                    appwindow: HWND,
                    message: *mut core::ffi::c_void,
                    riid: *const windows_core::GUID,
                    asyncoperation: *mut *mut core::ffi::c_void,
                ) -> windows_core::HRESULT {
                    unsafe {
                        let this: &Identity =
                            &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                        IUserConsentVerifierInterop_Impl::RequestVerificationForWindowAsync(
                            this,
                            core::mem::transmute_copy(&appwindow),
                            core::mem::transmute(&message),
                            core::mem::transmute_copy(&riid),
                            core::mem::transmute_copy(&asyncoperation),
                        )
                        .into()
                    }
                }
                Self {
                    base__: windows_core::IInspectable_Vtbl::new::<
                        Identity,
                        IUserConsentVerifierInterop,
                        OFFSET,
                    >(),
                    RequestVerificationForWindowAsync: RequestVerificationForWindowAsync::<
                        Identity,
                        OFFSET,
                    >,
                }
            }
            pub fn matches(iid: &windows_core::GUID) -> bool {
                iid == &<IUserConsentVerifierInterop as windows_core::Interface>::IID
            }
        }
        impl windows_core::RuntimeName for IUserConsentVerifierInterop {}
    }
}

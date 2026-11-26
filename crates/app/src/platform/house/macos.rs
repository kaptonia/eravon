pub use super::unix::{alert, console, downloads_dir, system_home};

pub fn open_url(url: &str) -> Result<(), String> {
    super::unix::run("open", &[url])
}

pub(super) fn panel_in(panel: *mut std::ffi::c_void, dir: &str) {
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2::msg_send;
    let Some(cls) = AnyClass::get(c"NSURL") else { return };
    if panel.is_null() {
        return;
    }
    let path = objc2_foundation::NSString::from_str(dir);
    unsafe {
        let url: *mut AnyObject = msg_send![cls, fileURLWithPath: &*path, isDirectory: true];
        if url.is_null() {
            return;
        }
        let p = &*(panel as *const AnyObject);
        let _: () = msg_send![p, setDirectoryURL: url];
    }
}

pub const COMMAND_WORD: &str = "key.commandMac";

pub const LINK_DELIVERY: &str = "openURLs";

pub fn arm_links() -> Result<(), String> {
    links::arm();
    Ok(())
}

pub(super) mod links {
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{AllocAnyThread, define_class, msg_send, sel};
    use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSObjectProtocol, NSString};

    const INTERNET_EVENT_CLASS: u32 = u32::from_be_bytes(*b"GURL");
    const AE_GET_URL: u32 = u32::from_be_bytes(*b"GURL");
    const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

    define_class!(
        // SAFETY: the superclass NSObject places no requirements on subclasses; this class does not implement Drop.
        #[unsafe(super(NSObject))]
        #[name = "EravonOpenUrlCatcher"]
        struct Catcher;

        impl Catcher {
            #[unsafe(method(handleGetURLEvent:withReplyEvent:))]
            fn handle_get_url(
                &self,
                event: &NSAppleEventDescriptor,
                _reply: &NSAppleEventDescriptor,
            ) {
                let desc: Option<Retained<NSAppleEventDescriptor>> = unsafe {
                    msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT]
                };
                let Some(d) = desc else { return };
                let s: Option<Retained<NSString>> = d.stringValue();
                if let Some(s) = s {
                    crate::house::openurl::push(s.to_string());
                }
            }
        }

        unsafe impl NSObjectProtocol for Catcher {}
    );

    pub fn arm() {
        let catcher: Retained<Catcher> = unsafe { msg_send![Catcher::alloc(), init] };
        let mgr = NSAppleEventManager::sharedAppleEventManager();
        let target: &AnyObject = &catcher;
        unsafe {
            let _: () = msg_send![
                &*mgr,
                setEventHandler: target,
                andSelector: sel!(handleGetURLEvent:withReplyEvent:),
                forEventClass: INTERNET_EVENT_CLASS,
                andEventID: AE_GET_URL,
            ];
        }
        std::mem::forget(catcher);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_three_four_byte_codes_are_the_ones_apple_names() {
            assert_eq!(INTERNET_EVENT_CLASS, 0x4755_524c);
            assert_eq!(AE_GET_URL, 0x4755_524c);
            assert_eq!(KEY_DIRECT_OBJECT, 0x2d2d_2d2d);
        }
    }
}

use std::sync::Arc;

use ash::{extensions::ext::DebugUtils, prelude::VkResult, vk, Entry, Instance};

pub struct DebugCallback {
    inner: vk::DebugUtilsMessengerEXT,
    debug_utils_loader: DebugUtils,
    // dependencies
    _instance: Arc<Instance>,
}

impl DebugCallback {
    pub fn new(
        entry: &Entry,
        instance: Arc<Instance>,
        debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
    ) -> VkResult<Self> {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(debug_callback);

        let debug_utils_loader = DebugUtils::new(entry, &instance);
        let inner = unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_info, None) }?;

        Ok(Self {
            inner,
            debug_utils_loader,
            _instance: instance,
        })
    }

    pub fn inner(&self) -> &vk::DebugUtilsMessengerEXT {
        &self.inner
    }
}

impl Drop for DebugCallback {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.inner, None);
        }
    }
}

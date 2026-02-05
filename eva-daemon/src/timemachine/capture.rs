use image::DynamicImage;
use screenshots::Screen;
use std::error::Error;

/// List of window titles that should never be captured
const BLOCKED_WINDOW_TITLES: &[&str] = &[
    // Browsers in private/incognito mode
    "incognito",
    "inprivate",
    "private browsing",
    "privado",
    "navegacao privada",
    // Banking and finance
    "bank",
    "banco",
    "paypal",
    "venmo",
    "stripe",
    "credit card",
    "cartao de credito",
    "nubank",
    "itau",
    "bradesco",
    "santander",
    "caixa",
    // Password managers
    "1password",
    "lastpass",
    "bitwarden",
    "keepass",
    "dashlane",
    // Crypto wallets
    "metamask",
    "ledger",
    "trezor",
    "coinbase",
    "binance",
    "wallet",
    "carteira",
    // Healthcare
    "patient",
    "medical",
    "health record",
    "prontuario",
    // Authentication
    "login",
    "sign in",
    "password",
    "senha",
    "authenticate",
    "2fa",
    "two-factor",
];

/// List of application names that should never be captured
const BLOCKED_APP_NAMES: &[&str] = &[
    "1password",
    "lastpass",
    "bitwarden",
    "keepassxc",
    "keychain",
    "credential",
    "authenticator",
    "authy",
];

/// Screen capture with privacy filtering
pub struct ScreenCapture {
    screens: Vec<Screen>,
    /// User-defined blocked window patterns
    user_blocked_patterns: Vec<String>,
    /// Whether privacy filter is enabled
    privacy_enabled: bool,
}

impl ScreenCapture {
    pub fn new() -> Self {
        let screens = Screen::all().unwrap_or_else(|_| vec![]);
        println!("[Capture] Found {} screen(s)", screens.len());

        Self {
            screens,
            user_blocked_patterns: Vec::new(),
            privacy_enabled: true,
        }
    }

    /// Add a user-defined pattern to block
    pub fn add_blocked_pattern(&mut self, pattern: String) {
        self.user_blocked_patterns.push(pattern.to_lowercase());
    }

    /// Enable or disable privacy filter
    pub fn set_privacy_enabled(&mut self, enabled: bool) {
        self.privacy_enabled = enabled;
    }

    /// Take a screenshot with privacy filtering
    pub fn take_screenshot(&self) -> Result<DynamicImage, Box<dyn Error>> {
        if self.screens.is_empty() {
            return Err("No screens found".into());
        }

        // Check privacy before capturing
        if self.privacy_enabled && self.should_block()? {
            return Err("Screenshot blocked by privacy filter".into());
        }

        // Capture primary screen
        let screen = self.screens[0];
        let image = screen.capture()?;

        let buffer = image.buffer();
        let dynamic_image = image::ImageBuffer::from_raw(image.width(), image.height(), buffer.clone())
            .map(DynamicImage::ImageRgba8)
            .ok_or("Failed to convert screenshot to image")?;

        Ok(dynamic_image)
    }

    /// Check if current screen should be blocked
    fn should_block(&self) -> Result<bool, Box<dyn Error>> {
        // Get active window information
        let active_window = self.get_active_window_info();

        if let Some((title, app_name)) = active_window {
            let title_lower = title.to_lowercase();
            let app_lower = app_name.to_lowercase();

            // Check against built-in blocked titles
            for blocked in BLOCKED_WINDOW_TITLES {
                if title_lower.contains(blocked) {
                    println!("[Privacy] Blocked: title contains '{}'", blocked);
                    return Ok(true);
                }
            }

            // Check against built-in blocked apps
            for blocked in BLOCKED_APP_NAMES {
                if app_lower.contains(blocked) {
                    println!("[Privacy] Blocked: app '{}' is in blocklist", app_lower);
                    return Ok(true);
                }
            }

            // Check against user-defined patterns
            for pattern in &self.user_blocked_patterns {
                if title_lower.contains(pattern) || app_lower.contains(pattern) {
                    println!("[Privacy] Blocked: matches user pattern '{}'", pattern);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get information about the currently active window
    #[cfg(target_os = "windows")]
    fn get_active_window_info(&self) -> Option<(String, String)> {
        // Try using active-win-pos-rs
        match active_win_pos_rs::get_active_window() {
            Ok(window) => {
                let title = window.title;
                let app_name = window.app_name;
                Some((title, app_name))
            }
            Err(_) => None,
        }
    }

    #[cfg(target_os = "linux")]
    fn get_active_window_info(&self) -> Option<(String, String)> {
        match active_win_pos_rs::get_active_window() {
            Ok(window) => {
                let title = window.title;
                let app_name = window.app_name;
                Some((title, app_name))
            }
            Err(_) => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn get_active_window_info(&self) -> Option<(String, String)> {
        match active_win_pos_rs::get_active_window() {
            Ok(window) => {
                let title = window.title;
                let app_name = window.app_name;
                Some((title, app_name))
            }
            Err(_) => None,
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    fn get_active_window_info(&self) -> Option<(String, String)> {
        // Redox OS or other - no window detection available
        None
    }
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_patterns() {
        // Test that blocked patterns are comprehensive
        let sensitive_titles = vec![
            "Bank of America - Login",
            "PayPal - Send Money",
            "1Password - Vault",
            "Chrome - Incognito",
            "Firefox Private Browsing",
            "Nubank - Conta",
        ];

        for title in sensitive_titles {
            let title_lower = title.to_lowercase();
            let should_block = BLOCKED_WINDOW_TITLES.iter()
                .any(|blocked| title_lower.contains(blocked));
            assert!(should_block, "Expected '{}' to be blocked", title);
        }
    }

    #[test]
    fn test_safe_patterns() {
        // Test that normal apps are not blocked
        let safe_titles = vec![
            "Visual Studio Code",
            "Terminal",
            "Spotify - Now Playing",
            "Discord - General",
        ];

        for title in safe_titles {
            let title_lower = title.to_lowercase();
            let should_block = BLOCKED_WINDOW_TITLES.iter()
                .any(|blocked| title_lower.contains(blocked));
            assert!(!should_block, "Expected '{}' to NOT be blocked", title);
        }
    }

    #[test]
    fn test_user_patterns() {
        let mut capture = ScreenCapture {
            screens: vec![],
            user_blocked_patterns: vec!["secret project".to_string()],
            privacy_enabled: true,
        };

        // Test that user patterns work
        assert!(capture.user_blocked_patterns.contains(&"secret project".to_string()));
    }

    #[test]
    fn test_add_blocked_pattern() {
        let mut capture = ScreenCapture {
            screens: vec![],
            user_blocked_patterns: vec![],
            privacy_enabled: true,
        };

        capture.add_blocked_pattern("MY_SECRET".to_string());
        // Should be lowercased
        assert!(capture.user_blocked_patterns.contains(&"my_secret".to_string()));
    }

    #[test]
    fn test_privacy_toggle() {
        let mut capture = ScreenCapture {
            screens: vec![],
            user_blocked_patterns: vec![],
            privacy_enabled: true,
        };

        assert!(capture.privacy_enabled);

        capture.set_privacy_enabled(false);
        assert!(!capture.privacy_enabled);

        capture.set_privacy_enabled(true);
        assert!(capture.privacy_enabled);
    }

    #[test]
    fn test_default_trait() {
        // Default should work
        let _capture: ScreenCapture = Default::default();
    }

    #[test]
    fn test_blocked_apps() {
        // Test that blocked apps are detected
        let sensitive_apps = vec![
            "1Password.exe",
            "LastPass",
            "Bitwarden",
            "KeePassXC",
            "Authenticator",
        ];

        for app in sensitive_apps {
            let app_lower = app.to_lowercase();
            let should_block = BLOCKED_APP_NAMES.iter()
                .any(|blocked| app_lower.contains(blocked));
            assert!(should_block, "Expected app '{}' to be blocked", app);
        }
    }

    #[test]
    fn test_crypto_wallets_blocked() {
        let wallet_titles = vec![
            "MetaMask - Ethereum Wallet",
            "Ledger Live",
            "Trezor Suite",
            "Coinbase - Buy Bitcoin",
            "Binance Trading",
        ];

        for title in wallet_titles {
            let title_lower = title.to_lowercase();
            let should_block = BLOCKED_WINDOW_TITLES.iter()
                .any(|blocked| title_lower.contains(blocked));
            assert!(should_block, "Expected crypto wallet '{}' to be blocked", title);
        }
    }

    #[test]
    fn test_brazilian_banks_blocked() {
        let bank_titles = vec![
            "Nubank - Sua Conta",
            "Itaú - Internet Banking",
            "Bradesco - NetEmpresa",
            "Santander - Pessoa Física",
            "Caixa - Internet Banking",
        ];

        for title in bank_titles {
            let title_lower = title.to_lowercase();
            let should_block = BLOCKED_WINDOW_TITLES.iter()
                .any(|blocked| title_lower.contains(blocked));
            assert!(should_block, "Expected Brazilian bank '{}' to be blocked", title);
        }
    }

    #[test]
    fn test_no_screens_error() {
        let capture = ScreenCapture {
            screens: vec![],
            user_blocked_patterns: vec![],
            privacy_enabled: false,
        };

        let result = capture.take_screenshot();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No screens found"));
    }
}

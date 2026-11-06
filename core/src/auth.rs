//! Account registration, login and "remember me".
//!
//! This is a LOCAL account store (files on this computer). The API is shaped
//! like a client talking to a login server, so swapping in a network backend
//! later only means re-implementing `AccountStore`'s methods.

use crate::crypto::{self, PBKDF2_ITERATIONS};
use crate::save::{self, Account, Settings};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const MIN_PASSWORD_LEN: usize = 8;
const MAX_FAILED_ATTEMPTS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    InvalidEmail,
    PasswordTooShort,
    PasswordMismatch,
    EmailTaken,
    WrongCredentials,
    LockedOut(u64),
    Io(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidEmail => write!(f, "Please enter a valid e-mail address."),
            AuthError::PasswordTooShort => write!(f, "Password must be at least {} characters.", MIN_PASSWORD_LEN),
            AuthError::PasswordMismatch => write!(f, "Passwords don't match."),
            AuthError::EmailTaken => write!(f, "An account with that e-mail already exists."),
            AuthError::WrongCredentials => write!(f, "E-mail or password is incorrect."),
            AuthError::LockedOut(s) => write!(f, "Too many attempts. Try again in {}s.", s),
            AuthError::Io(e) => write!(f, "Could not access save data: {}", e),
        }
    }
}

pub fn normalize_email(e: &str) -> String {
    e.trim().to_lowercase()
}

pub fn valid_email(e: &str) -> bool {
    let e = e.trim();
    if e.len() < 5 || e.len() > 254 || e.contains(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = e.split_once('@') else { return false };
    !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
}

pub struct AccountStore {
    pub dir: PathBuf,
    failed: u32,
    locked_until: Option<Instant>,
}

/// What's stored on disk when "remember me" is ticked: e-mail + random token.
/// The account keeps only a hash of the token, never the password.
#[derive(Debug, Clone)]
pub struct Remembered {
    pub email: String,
    token_hex: String,
}

impl AccountStore {
    pub fn new(dir: PathBuf) -> Self {
        AccountStore { dir, failed: 0, locked_until: None }
    }
    pub fn default_location() -> Self {
        Self::new(save::data_dir())
    }

    fn account_path(&self, email: &str) -> PathBuf {
        let id = crypto::to_hex(&crypto::sha256(normalize_email(email).as_bytes()));
        self.dir.join("accounts").join(format!("{}.acc", &id[..32]))
    }
    fn remember_path(&self) -> PathBuf {
        self.dir.join("remember.txt")
    }

    pub fn exists(&self, email: &str) -> bool {
        self.account_path(email).exists()
    }

    pub fn load(&self, email: &str) -> Result<Account, AuthError> {
        let txt = std::fs::read_to_string(self.account_path(email)).map_err(|_| AuthError::WrongCredentials)?;
        save::decode_account(&txt).map_err(AuthError::Io)
    }

    pub fn save(&self, acc: &Account) -> Result<(), AuthError> {
        save::write_atomic(&self.account_path(&acc.email), &save::encode_account(acc))
            .map_err(|e| AuthError::Io(e.to_string()))
    }

    pub fn register(&mut self, email: &str, password: &str, confirm: &str) -> Result<Account, AuthError> {
        let email = normalize_email(email);
        if !valid_email(&email) {
            return Err(AuthError::InvalidEmail);
        }
        if password.chars().count() < MIN_PASSWORD_LEN {
            return Err(AuthError::PasswordTooShort);
        }
        if password != confirm {
            return Err(AuthError::PasswordMismatch);
        }
        if self.exists(&email) {
            return Err(AuthError::EmailTaken);
        }
        let salt = crypto::random_bytes(16);
        let hash = crypto::pbkdf2_sha256(password.as_bytes(), &salt, PBKDF2_ITERATIONS);
        let acc = Account {
            email,
            salt_hex: crypto::to_hex(&salt),
            hash_hex: crypto::to_hex(&hash),
            iterations: PBKDF2_ITERATIONS,
            remember_hash: None,
            settings: Settings::default(),
            characters: Vec::new(),
            created_unix: now_unix(),
        };
        self.save(&acc)?;
        Ok(acc)
    }

    pub fn login(&mut self, email: &str, password: &str) -> Result<Account, AuthError> {
        if let Some(t) = self.locked_until {
            let now = Instant::now();
            if now < t {
                return Err(AuthError::LockedOut((t - now).as_secs() + 1));
            }
            self.locked_until = None;
            self.failed = 0;
        }
        let result = self.check(email, password);
        if result.is_err() {
            self.failed += 1;
            if self.failed >= MAX_FAILED_ATTEMPTS {
                self.locked_until = Some(Instant::now() + LOCKOUT);
            }
        } else {
            self.failed = 0;
        }
        result
    }

    fn check(&self, email: &str, password: &str) -> Result<Account, AuthError> {
        let email = normalize_email(email);
        if !valid_email(&email) {
            return Err(AuthError::InvalidEmail);
        }
        let acc = self.load(&email)?;
        let salt = crypto::from_hex(&acc.salt_hex).ok_or(AuthError::WrongCredentials)?;
        let expected = crypto::from_hex(&acc.hash_hex).ok_or(AuthError::WrongCredentials)?;
        let got = crypto::pbkdf2_sha256(password.as_bytes(), &salt, acc.iterations);
        if crypto::ct_eq(&got, &expected) {
            Ok(acc)
        } else {
            Err(AuthError::WrongCredentials)
        }
    }

    // ---------------------------------------------------- remember me

    /// Server side of "remember me": make a fresh token for this account,
    /// keep only its hash, and return the token for the player's computer.
    pub fn issue_token(&self, acc: &mut Account) -> Result<String, AuthError> {
        let token = crypto::random_bytes(32);
        acc.remember_hash = Some(crypto::to_hex(&crypto::sha256(&token)));
        self.save(acc)?;
        Ok(crypto::to_hex(&token))
    }

    /// Server side: log in with a token from `issue_token`.
    pub fn login_token(&self, email: &str, token_hex: &str) -> Result<Account, AuthError> {
        let acc = self.load(&normalize_email(email))?;
        let token = crypto::from_hex(token_hex).ok_or(AuthError::WrongCredentials)?;
        let h = crypto::to_hex(&crypto::sha256(&token));
        match &acc.remember_hash {
            Some(stored) if crypto::ct_eq(stored.as_bytes(), h.as_bytes()) => Ok(acc),
            _ => Err(AuthError::WrongCredentials),
        }
    }

    /// Every account on disk (the server uses this to keep names unique).
    pub fn all_accounts(&self) -> Vec<Account> {
        let Ok(rd) = std::fs::read_dir(self.dir.join("accounts")) else { return Vec::new() };
        rd.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "acc").unwrap_or(false))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|t| save::decode_account(&t).ok())
            .collect()
    }

    /// Create a fresh token for this account and store it on this computer.
    pub fn remember(&self, acc: &mut Account) -> Result<(), AuthError> {
        let token = crypto::random_bytes(32);
        acc.remember_hash = Some(crypto::to_hex(&crypto::sha256(&token)));
        self.save(acc)?;
        let body = format!(
            "# Galebound 'remember me' data. Delete this file to sign out on this computer.\nemail={}\ntoken={}\n",
            acc.email,
            crypto::to_hex(&token)
        );
        save::write_atomic(&self.remember_path(), &body).map_err(|e| AuthError::Io(e.to_string()))
    }

    /// Remove the stored login from this computer and invalidate the token.
    pub fn forget(&self, acc: Option<&mut Account>) {
        let _ = std::fs::remove_file(self.remember_path());
        if let Some(acc) = acc {
            acc.remember_hash = None;
            let _ = self.save(acc);
        }
    }

    pub fn remembered(&self) -> Option<Remembered> {
        let txt = std::fs::read_to_string(self.remember_path()).ok()?;
        let mut email = None;
        let mut token = None;
        for l in txt.lines() {
            if let Some(v) = l.strip_prefix("email=") {
                email = Some(v.trim().to_string());
            }
            if let Some(v) = l.strip_prefix("token=") {
                token = Some(v.trim().to_string());
            }
        }
        Some(Remembered { email: email?, token_hex: token? })
    }

    /// Log in with the remembered token. Fails (and clears it) if the token
    /// was revoked, e.g. by signing out elsewhere.
    pub fn login_remembered(&self, r: &Remembered) -> Result<Account, AuthError> {
        let acc = self.load(&r.email)?;
        let token = crypto::from_hex(&r.token_hex).ok_or(AuthError::WrongCredentials)?;
        let h = crypto::to_hex(&crypto::sha256(&token));
        match &acc.remember_hash {
            Some(stored) if crypto::ct_eq(stored.as_bytes(), h.as_bytes()) => Ok(acc),
            _ => {
                let _ = std::fs::remove_file(self.remember_path());
                Err(AuthError::WrongCredentials)
            }
        }
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store(tag: &str) -> AccountStore {
        let d = std::env::temp_dir().join(format!("galebound_test_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        AccountStore::new(d)
    }

    #[test]
    fn email_validation() {
        assert!(valid_email("skeng@example.com"));
        assert!(!valid_email("nope"));
        assert!(!valid_email("a@b"));
        assert!(!valid_email("a b@c.com"));
        assert!(!valid_email("a@@c.com"));
    }

    #[test]
    fn register_login_remember() {
        let mut s = tmp_store("auth");
        assert_eq!(s.register("x@y.com", "short", "short").unwrap_err(), AuthError::PasswordTooShort);
        assert_eq!(s.register("x@y.com", "longenough", "different").unwrap_err(), AuthError::PasswordMismatch);
        s.register(" X@Y.com ", "longenough", "longenough").unwrap();
        assert_eq!(s.register("x@y.com", "longenough", "longenough").unwrap_err(), AuthError::EmailTaken);
        assert_eq!(s.login("x@y.com", "wrongpass").unwrap_err(), AuthError::WrongCredentials);
        let mut acc = s.login("X@y.com", "longenough").unwrap();
        s.remember(&mut acc).unwrap();
        let r = s.remembered().unwrap();
        assert_eq!(r.email, "x@y.com");
        assert!(s.login_remembered(&r).is_ok());
        // The password itself never appears on disk.
        let all: String = std::fs::read_dir(s.dir.join("accounts"))
            .unwrap()
            .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
            .collect();
        assert!(!all.contains("longenough"));
        s.forget(Some(&mut acc));
        assert!(s.remembered().is_none());
        assert!(s.login_remembered(&r).is_err(), "revoked token rejected");
        let _ = std::fs::remove_dir_all(&s.dir);
    }

    #[test]
    fn lockout_after_failures() {
        let mut s = tmp_store("lock");
        s.register("l@y.com", "longenough", "longenough").unwrap();
        for _ in 0..5 {
            let _ = s.login("l@y.com", "bad-password");
        }
        assert!(matches!(s.login("l@y.com", "longenough"), Err(AuthError::LockedOut(_))));
        let _ = std::fs::remove_dir_all(&s.dir);
    }
}

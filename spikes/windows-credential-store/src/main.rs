use moe_credential_store::{
    CredentialStore, PlatformCredentialStore, RelayCredentialId, SecretBytes,
};
use std::{
    env,
    error::Error,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const ACCOUNT_ENV: &str = "MOE_CREDENTIAL_PROBE_ACCOUNT";
const SECRET_ENV: &str = "MOE_CREDENTIAL_PROBE_SECRET";

struct CleanupCredential(RelayCredentialId);

impl Drop for CleanupCredential {
    fn drop(&mut self) {
        let _ = PlatformCredentialStore.delete(&self.0);
    }
}

fn child_value(name: &str) -> Result<String, Box<dyn Error>> {
    let value = env::var(name)?;
    // SAFETY: probe child is single-threaded and removes only its own private variable.
    unsafe { env::remove_var(name) };
    Ok(value)
}

fn child(mode: &str) -> Result<(), Box<dyn Error>> {
    let id = RelayCredentialId::new(child_value(ACCOUNT_ENV)?)?;
    let store = PlatformCredentialStore;
    match mode {
        "write" => {
            let mut raw = child_value(SECRET_ENV)?.into_bytes();
            let credential = SecretBytes::new(raw.clone())?;
            raw.fill(0);
            store.store(&id, &credential)?;
            println!("WRITE_OK");
        }
        "verify" | "verify-not" => {
            let mut expected = child_value(SECRET_ENV)?.into_bytes();
            let actual = store.load(&id)?.ok_or("credential missing")?;
            let matches = actual.expose() == expected;
            expected.fill(0);
            if (mode == "verify" && !matches) || (mode == "verify-not" && matches) {
                return Err("credential comparison failed".into());
            }
            println!("COMPARE_OK");
        }
        "delete" => {
            if !store.delete(&id)? {
                return Err("credential was already missing".into());
            }
            println!("DELETE_OK");
        }
        "missing" => {
            if store.contains(&id)? {
                return Err("credential still exists".into());
            }
            println!("MISSING_OK");
        }
        _ => return Err("unknown child mode".into()),
    }
    Ok(())
}

fn run_child(mode: &str, account_id: &str, secret: Option<&str>) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(env::current_exe()?);
    command
        .arg("--child")
        .arg(mode)
        .env(ACCOUNT_ENV, account_id)
        .env_remove(SECRET_ENV);
    if let Some(secret) = secret {
        command.env(SECRET_ENV, secret);
    }
    Ok(command.output()?)
}

fn require_success(
    output: Output,
    secrets: &[&str],
    captured: &mut Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    captured.extend_from_slice(&output.stdout);
    captured.extend_from_slice(&output.stderr);
    for secret in secrets {
        if output
            .stdout
            .windows(secret.len())
            .any(|part| part == secret.as_bytes())
            || output
                .stderr
                .windows(secret.len())
                .any(|part| part == secret.as_bytes())
        {
            return Err("child output leaked a secret".into());
        }
    }
    if !output.status.success() {
        return Err(format!("credential probe child failed with {}", output.status).into());
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let account_id = format!("probe-{}-{nonce:x}", std::process::id());
    let id = RelayCredentialId::new(account_id.clone())?;
    let _cleanup = CleanupCredential(id.clone());
    let secret_one = format!("device-alpha-{}-{nonce:x}", std::process::id());
    let secret_two = format!(
        "device-beta-{}-{:x}",
        std::process::id(),
        nonce.rotate_left(17)
    );
    let secrets = [secret_one.as_str(), secret_two.as_str()];
    let mut captured = Vec::new();
    let store = PlatformCredentialStore;

    let _ = store.delete(&id)?;
    require_success(
        run_child("missing", &account_id, None)?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("write", &account_id, Some(&secret_one))?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("verify", &account_id, Some(&secret_one))?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("write", &account_id, Some(&secret_two))?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("verify", &account_id, Some(&secret_two))?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("verify-not", &account_id, Some(&secret_one))?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("delete", &account_id, None)?,
        &secrets,
        &mut captured,
    )?;
    require_success(
        run_child("missing", &account_id, None)?,
        &secrets,
        &mut captured,
    )?;

    if store.contains(&id)? {
        return Err("credential cleanup verification failed".into());
    }

    println!(
        "{{\n  \"result\": \"PASS\",\n  \"backend\": \"moe-credential-store / Windows Credential Manager\",\n  \"targetSchema\": \"M.O.E./relay-device/v1/<account-id>\",\n  \"separateProcessRead\": true,\n  \"write\": \"PASS\",\n  \"update\": \"PASS\",\n  \"delete\": \"PASS\",\n  \"secretOnCommandLine\": false,\n  \"secretInChildOutput\": false,\n  \"probeCredentialRemoved\": true\n}}"
    );
    captured.fill(0);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("--child") {
        return child(arguments.get(2).ok_or("missing child mode")?);
    }
    run()
}

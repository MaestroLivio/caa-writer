//! Politica di accesso e prova senza effetti.
//!
//! La politica è applicata nel dispatch condiviso, quindi vale allo stesso
//! modo per CLI, server MCP e bridge della GUI. Non è una descrizione
//! cosmetica: un metodo che modifica dati viene rifiutato in modalità `read`.

use serde_json::{json, Value};

use super::registry::{self, Operation};

/// Livello di accesso richiesto da chi pilota l'applicazione.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Solo lettura: nessuna modifica a database, file o preferenze.
    Read,
    /// Lettura e modifiche non distruttive.
    Write,
    /// Tutto, comprese le operazioni distruttive (che richiedono `confirm`).
    Admin,
}

impl AccessMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "read" | "ro" => Ok(Self::Read),
            "write" | "rw" => Ok(Self::Write),
            "admin" => Ok(Self::Admin),
            other => Err(format!(
                "modalità di accesso non valida: {other} (usa read, write o admin)"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DispatchOptions {
    pub access: AccessMode,
    pub dry_run: bool,
}

impl Default for DispatchOptions {
    /// Il comportamento storico della CLI locale è amministrativo. I client
    /// remoti (MCP) scelgono esplicitamente una modalità più stretta.
    fn default() -> Self {
        Self {
            access: AccessMode::Admin,
            dry_run: false,
        }
    }
}

impl DispatchOptions {
    pub fn read_only() -> Self {
        Self {
            access: AccessMode::Read,
            dry_run: false,
        }
    }

    pub fn with_access(mut self, access: AccessMode) -> Self {
        self.access = access;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

/// Verifica preliminare. Restituisce `Ok(None)` se il metodo può procedere,
/// `Ok(Some(value))` se è un dry-run senza effetti, `Err` se è vietato.
pub fn check(
    operation: &Operation,
    params: &Value,
    options: DispatchOptions,
) -> Result<Option<Value>, String> {
    if operation.mutating && options.access == AccessMode::Read {
        return Err(format!(
            "operazione negata: {} modifica dati e la modalità di accesso è read",
            operation.name
        ));
    }
    if operation.destructive {
        if options.access != AccessMode::Admin {
            return Err(format!(
                "operazione negata: {} è distruttiva e richiede la modalità admin",
                operation.name
            ));
        }
        let confirmed = params
            .get("confirm")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if !confirmed {
            return Err(format!(
                "operazione distruttiva non confermata: ripeti con \"confirm\": true ({})",
                operation.name
            ));
        }
    }
    if options.dry_run && operation.mutating {
        return Ok(Some(json!({
            "dry_run": true,
            "method": operation.name,
            "tool": operation.mcp,
            "description": operation.description,
            "mutating": operation.mutating,
            "destructive": operation.destructive,
            "access": options.access.as_str(),
            "params": params,
            "would_execute": true,
        })));
    }
    Ok(None)
}

/// Verifica che una modalità dichiarata dal client sia compatibile con
/// l'operazione richiesta. Usata dai test e dal server MCP.
pub fn can_run(method: &str, access: AccessMode) -> bool {
    match registry::find(method) {
        Some(operation) => {
            if operation.destructive {
                access == AccessMode::Admin
            } else if operation.mutating {
                access != AccessMode::Read
            } else {
                true
            }
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_mode_rejects_mutations() {
        let operation = registry::find("settings.set").unwrap();
        let error = check(operation, &json!({}), DispatchOptions::read_only()).unwrap_err();
        assert!(error.contains("read"));
    }

    #[test]
    fn destructive_requires_admin_and_confirmation() {
        let operation = registry::find("project.delete").unwrap();
        let write = DispatchOptions::default().with_access(AccessMode::Write);
        assert!(check(operation, &json!({"confirm": true}), write).is_err());
        let no_confirm = DispatchOptions::default();
        assert!(check(operation, &json!({}), no_confirm).is_err());
        let confirmed = DispatchOptions::default();
        let outcome = check(operation, &json!({"confirm": true}), confirmed).unwrap();
        assert!(outcome.is_none());
    }

    #[test]
    fn dry_run_skips_mutations() {
        let operation = registry::find("project.create").unwrap();
        let outcome = check(operation, &json!({}), DispatchOptions::default().with_dry_run(true))
            .unwrap()
            .unwrap();
        assert_eq!(outcome["dry_run"], true);
        assert_eq!(outcome["method"], "project.create");
    }
}

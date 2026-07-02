//! Shared Wasmtime engine and blake3-keyed module cache (inode-cache analogue).

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use wasmtime::{Config, Engine, Module};

/// Process-wide Wasmtime engine with compiled-module cache.
pub struct WasmEngine {
    engine: Engine,
    modules: Mutex<HashMap<[u8; 32], Module>>,
}

static SHARED: OnceLock<Arc<WasmEngine>> = OnceLock::new();

/// Default instruction fuel budget (~10 ms compute on modern hardware).
pub fn default_fuel_limit() -> u64 {
    std::env::var("AUTONOMIC_WASM_FUEL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500_000_000)
}

/// Lazily initialized shared engine (compile once, reuse).
pub fn shared() -> Arc<WasmEngine> {
    SHARED
        .get_or_init(|| {
            let mut config = Config::new();
            config.consume_fuel(true);
            Arc::new(WasmEngine {
                engine: Engine::new(&config).expect("wasmtime engine init"),
                modules: Mutex::new(HashMap::new()),
            })
        })
        .clone()
}

impl WasmEngine {
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Load or compile a module; cache hit avoids JIT recompile.
    pub fn get_or_compile(&self, wasm_bytes: &[u8]) -> Result<Module> {
        let hash = *blake3::hash(wasm_bytes).as_bytes();
        if let Some(module) = self.modules.lock().unwrap().get(&hash) {
            return Ok(module.clone());
        }
        let module = Module::new(&self.engine, wasm_bytes)
            .with_context(|| format!("compile wasm module ({} bytes)", wasm_bytes.len()))?;
        self.modules.lock().unwrap().insert(hash, module.clone());
        Ok(module)
    }

    /// Drop cached modules (tests / memory pressure).
    pub fn clear_cache(&self) {
        self.modules.lock().unwrap().clear();
    }

    #[must_use]
    pub fn cache_len(&self) -> usize {
        self.modules.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ECHO_WAT: &str = r#"
        (module
          (func (export "run") (result i32)
            i32.const 42))
    "#;

    #[test]
    fn module_cache_hit_on_second_compile() {
        let engine = shared();
        engine.clear_cache();
        let bytes = ECHO_WAT.as_bytes();
        assert_eq!(engine.cache_len(), 0);
        let _ = engine.get_or_compile(bytes).expect("first compile");
        assert_eq!(engine.cache_len(), 1);
        let _ = engine.get_or_compile(bytes).expect("cache hit");
        assert_eq!(engine.cache_len(), 1);
    }
}

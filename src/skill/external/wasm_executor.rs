use crate::error::{Result, AgentHubError};
use crate::skill::{ExecutionContext, SkillExecutor, SkillResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wasmtime::{Engine, Module, Store, Linker, Memory, Config};
use wasmtime_wasi::WasiCtxBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSkillConfig {
    pub max_memory_mb: usize,
    pub max_execution_time_secs: u64,
    pub allow_network: bool,
    pub allowed_dirs: Vec<PathBuf>,
}

impl Default for WasmSkillConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 256,
            max_execution_time_secs: 30,
            allow_network: false,
            allowed_dirs: vec![],
        }
    }
}

pub struct WasmSkillExecutor {
    engine: Engine,
    module: Module,
    config: WasmSkillConfig,
    #[allow(dead_code)]
    wasm_path: PathBuf,
}

impl WasmSkillExecutor {
    pub fn new(wasm_path: PathBuf, config: WasmSkillConfig) -> Result<Self> {
        if !wasm_path.exists() {
            return Err(AgentHubError::FileNotFound { 
                path: wasm_path.to_string_lossy().to_string() 
            });
        }

        let mut wasmtime_config = Config::new();
        wasmtime_config.async_support(true);
        
        let engine = Engine::new(&wasmtime_config)
            .map_err(|e| AgentHubError::Internal(format!("Failed to create WASM engine: {}", e)))?;
        let module = Module::from_file(&engine, &wasm_path)
            .map_err(|e| AgentHubError::Internal(format!("Failed to load WASM module: {}", e)))?;

        Ok(Self {
            engine,
            module,
            config,
            wasm_path,
        })
    }

    fn create_store_and_linker(&self, _context: &ExecutionContext) -> Result<(Store<wasmtime_wasi::WasiCtx>, Linker<wasmtime_wasi::WasiCtx>)> {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();

        let mut store = Store::new(&self.engine, wasi);
        store.set_epoch_deadline(self.config.max_execution_time_secs);

        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)
            .map_err(|e| AgentHubError::Internal(format!("Failed to add WASI to linker: {}", e)))?;

        Ok((store, linker))
    }
}

#[async_trait]
impl SkillExecutor for WasmSkillExecutor {
    async fn execute(
        &self,
        params: serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<SkillResult> {
        let input_bytes = serde_json::to_vec(&params)
            .map_err(|e| AgentHubError::Internal(format!("Failed to serialize input: {}", e)))?;

        let (mut store, linker) = self.create_store_and_linker(context)?;

        let instance = linker.instantiate_async(&mut store, &self.module)
            .await
            .map_err(|e| AgentHubError::Internal(format!("Failed to instantiate WASM module: {}", e)))?;

        let execute_fn = instance.get_func(&mut store, "execute")
            .ok_or_else(|| AgentHubError::Internal("WASM module missing 'execute' function".to_string()))?;

        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| AgentHubError::Internal("WASM module missing 'memory' export".to_string()))?;

        let input_ptr = self.allocate_memory(&mut store, &memory, input_bytes.len() as u64)
            .map_err(|e| AgentHubError::Internal(format!("Failed to allocate WASM memory: {}", e)))?;

        memory.write(&mut store, input_ptr as usize, &input_bytes)
            .map_err(|e| AgentHubError::Internal(format!("Failed to write to WASM memory: {}", e)))?;

        let mut results = [wasmtime::Val::I32(0)];
        execute_fn.call_async(&mut store, &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input_bytes.len() as i32)], &mut results)
            .await
            .map_err(|e| AgentHubError::Internal(format!("WASM execution failed: {}", e)))?;

        let result_ptr = results[0].i32().ok_or_else(|| AgentHubError::Internal("WASM execution returned invalid result".to_string()))?;

        let result_len = self.read_u32_from_memory(&mut store, &memory, result_ptr as usize)
            .map_err(|e| AgentHubError::Internal(format!("Failed to read result length: {}", e)))?;

        let mut result_bytes = vec![0u8; result_len as usize];
        memory.read(&mut store, (result_ptr + 8) as usize, &mut result_bytes)
            .map_err(|e| AgentHubError::Internal(format!("Failed to read result data: {}", e)))?;

        let result_str = String::from_utf8(result_bytes)
            .map_err(|e| AgentHubError::Internal(format!("Invalid UTF-8 in WASM result: {}", e)))?;

        let skill_result: SkillResult = serde_json::from_str(&result_str)
            .map_err(|e| AgentHubError::Internal(format!("Failed to parse WASM result: {}", e)))?;

        self.deallocate_memory(&mut store, &memory, input_ptr, input_bytes.len() as u64);

        Ok(skill_result)
    }
}

impl WasmSkillExecutor {
    fn allocate_memory(&self, store: &mut Store<wasmtime_wasi::WasiCtx>, memory: &Memory, size: u64) -> Result<i32> {
        let alloc_fn = memory.grow(store, (size + 65535) / 65536);
        let base_page = alloc_fn.map_err(|e| AgentHubError::Internal(format!("Memory allocation failed: {}", e)))?;
        Ok((base_page * 65536) as i32)
    }

    fn deallocate_memory(&self, _store: &mut Store<wasmtime_wasi::WasiCtx>, _memory: &Memory, _ptr: i32, _size: u64) {
        // WASM doesn't support explicit deallocation, memory will be reclaimed when store is dropped
    }

    fn read_u32_from_memory(&self, store: &mut Store<wasmtime_wasi::WasiCtx>, memory: &Memory, offset: usize) -> Result<u32> {
        let mut buf = [0u8; 4];
        memory.read(store, offset, &mut buf)
            .map_err(|e| AgentHubError::Internal(format!("Memory read failed: {}", e)))?;
        Ok(u32::from_le_bytes(buf))
    }
}

# Skill Hello World - 开发者指南

> 🚀 AgentHub WASM Skill 开发入门示例

## 快速开始

### 1. 环境准备

安装 Rust 和 WASM 编译目标：

```bash
# 安装 Rust（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 WASM 编译目标
rustup target add wasm32-wasip1
```

### 2. 编译 Skill

```bash
cd examples/skill-hello-world
cargo build --target wasm32-wasip1 --release
```

编译产物位于：
```
target/wasm32-wasip1/release/skill_hello_world.wasm
```

### 3. 运行 Skill

将编译好的 `.wasm` 文件放入 AgentHub 的 skills 目录，然后通过 CLI 运行：

```bash
# 问候操作
agenthub skill run example.hello.world '{"operation": "greet", "name": "Alice"}'

# 回显操作
agenthub skill run example.hello.world '{"operation": "echo", "name": "test_data"}'

# 数学计算
agenthub skill run example.hello.world '{"operation": "math", "a": 3.5, "b": 2.1}'
```

## 项目结构

```
skill-hello-world/
├── Cargo.toml          # Rust 项目配置（含 WASM 编译设置）
├── manifest.yaml       # Skill 元数据（名称、版本、输入输出格式）
├── SKILL.md            # 人类可读的 Skill 说明
├── README.md           # 本文件 - 开发者指南
└── src/
    └── lib.rs          # Skill 核心实现（含详细注释）
```

## 核心概念

### WASM Skill 接口

WASM Skill 通过 FFI（外部函数接口）与 AgentHub 宿主环境通信：

```
┌─────────────────┐         ┌─────────────────┐
│  AgentHub 宿主   │ ──────▶ │  WASM Skill     │
│                 │  输入    │                 │
│  (Rust/wasmtime) │  JSON   │  (Rust/WASM)    │
│                 │ ◀────── │                 │
└─────────────────┘  输出    └─────────────────┘
```

### 数据传递方式

**输入**：宿主将 JSON 参数写入 WASM 内存，传递指针和长度
```
execute(input_ptr: i32, input_len: i32) -> i32
```

**输出**：Skill 将 JSON 结果写入 WASM 内存，返回结果指针
```
内存布局: [4字节长度][4字节填充][N字节JSON数据]
          ^ ptr        ^ ptr+4      ^ ptr+8
```

### 标准输出格式

所有 Skill 必须返回以下 JSON 结构：

```json
{
  "output": { ... },
  "files_created": [],
  "warnings": []
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `output` | JSON 值 | 主要输出内容 |
| `files_created` | 字符串数组 | 创建的文件路径 |
| `warnings` | 字符串数组 | 警告信息 |

## 开发你自己的 Skill

### 步骤 1：创建项目

```bash
cargo new --lib my-skill
cd my-skill
```

### 步骤 2：配置 Cargo.toml

```toml
[package]
name = "my-skill"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # 必须！

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 步骤 3：实现 execute 函数

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Input {
    message: String,
}

#[derive(Serialize)]
struct Output {
    output: serde_json::Value,
    files_created: Vec<String>,
    warnings: Vec<String>,
}

#[no_mangle]
pub extern "C" fn execute(input_ptr: i32, input_len: i32) -> i32 {
    // 1. 读取输入
    let input_bytes = unsafe {
        std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize)
    };
    
    // 2. 解析 JSON
    let input: Input = serde_json::from_slice(input_bytes).unwrap();
    
    // 3. 执行逻辑
    let result = Output {
        output: serde_json::json!({
            "response": format!("收到: {}", input.message)
        }),
        files_created: vec![],
        warnings: vec![],
    };
    
    // 4. 返回结果
    allocate_result(&result)
}

fn allocate_result(result: &Output) -> i32 {
    let json = serde_json::to_string(result).unwrap();
    let len = json.len();
    let ptr = unsafe {
        let p = libc::malloc((len + 8) as usize) as i32;
        std::ptr::write(p as *mut i32, len as i32);
        std::ptr::copy_nonoverlapping(json.as_ptr(), (p + 8) as *mut u8, len);
        p
    };
    ptr
}
```

### 步骤 4：编译

```bash
cargo build --target wasm32-wasip1 --release
```

## 测试 Skill

### 基础功能测试

```bash
# 问候操作
agenthub skill run example.hello.world '{"operation": "greet", "name": "Alice"}'

# 回显操作
agenthub skill run example.hello.world '{"operation": "echo", "name": "test_data"}'

# 数学计算
agenthub skill run example.hello.world '{"operation": "math", "a": 3.5, "b": 2.1}'
```

### 沙箱安全测试

WASM Skill 运行在 wasmtime 沙箱中，以下测试用于验证隔离能力：

| 测试操作 | 说明 | 预期结果 |
|----------|------|----------|
| `test_fs_read` | 尝试读取系统文件 | **PASS**: 沙箱阻止文件访问 |
| `test_fs_write` | 尝试写入系统目录 | **PASS**: 沙箱阻止文件写入 |
| `test_env` | 尝试访问环境变量 | **PASS**: 环境变量受限 |
| `test_network` | 尝试建立 TCP 连接 | **PASS**: 沙箱阻止网络访问 |

运行安全测试：

```bash
# 测试文件系统读取隔离
agenthub skill run example.hello.world '{"operation": "test_fs_read"}'

# 测试文件系统写入隔离
agenthub skill run example.hello.world '{"operation": "test_fs_write"}'

# 测试环境变量隔离
agenthub skill run example.hello.world '{"operation": "test_env"}'

# 测试网络隔离
agenthub skill run example.hello.world '{"operation": "test_network"}'
```

预期输出示例：
```json
{
  "output": {
    "test": "文件系统读取",
    "result": "PASS: 沙箱生效！无法读取系统文件: ..."
  },
  "files_created": [],
  "warnings": []
}
```

## 常见问题

### Q: 编译时提示找不到 `wasm32-wasip1` 目标？

```bash
rustup target add wasm32-wasip1
```

### Q: 为什么需要 `crate-type = ["cdylib"]`？

WASM 模块需要编译为动态库格式（cdylib），否则 cargo 会编译为 Rust 静态库（rlib），WASM 加载器无法识别。

### Q: 可以使用异步代码吗？

WASM Skill 运行在同步执行环境中，不支持 async/await。如果需要异步操作，应通过宿主环境提供的接口实现。

### Q: 内存如何管理？

Skill 通过 `libc::malloc` 分配的内存由宿主环境负责释放。WASM 模块卸载时，所有内存自动回收。

## 参考资料

- [AgentHub 项目规范和需求文档](../../docs/项目规范和需求文档.md)
- [AgentHub 开发实战指南](../../docs/开发实战指南.md)
- [WASM 规范](https://webassembly.org/)
- [wasmtime 文档](https://docs.wasmtime.dev/)

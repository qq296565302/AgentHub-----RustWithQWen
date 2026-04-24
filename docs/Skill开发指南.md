# AgentHub Skill 开发指南

> **版本**: v1.0  
> **日期**: 2026-04-24  
> **适用对象**: Skill 开发者、贡献者

---

## 目录

1. [Skill 概述](#1-skill-概述)
2. [Skill 类型](#2-skill-类型)
3. [开发环境准备](#3-开发环境准备)
4. [WASM Skill 开发](#4-wasm-skill-开发)
5. [接口规范详解](#5-接口规范详解)
6. [内存管理](#6-内存管理)
7. [安全沙箱](#7-安全沙箱)
8. [测试策略](#8-测试策略)
9. [发布与部署](#9-发布与部署)
10. [最佳实践](#10-最佳实践)
11. [常见问题](#11-常见问题)

---

## 1. Skill 概述

Skill 是 AgentHub 平台的可插拔功能模块。每个 Skill 实现特定的能力（如代码解释、测试生成、文本处理等），通过统一的接口被宿主环境调用。

### 1.1 核心特性

- **沙箱隔离**: WASM Skill 运行在 wasmtime 沙箱中，与宿主环境隔离
- **语言无关**: 理论上支持任何可编译为 WASM 的语言（目前以 Rust 为主）
- **热插拔**: Skill 可以动态加载和卸载，无需重启宿主
- **安全可控**: 内存、超时、网络、文件系统访问均受宿主控制

### 1.2 架构概览

```
┌─────────────────────────────────────────────────┐
│                   AgentHub 宿主                   │
│  ┌──────────┐  ┌──────────────┐  ┌───────────┐  │
│  │ CLI REPL │  │ HTTP Server  │  │ 审计系统  │  │
│  └────┬─────┘  └──────┬───────┘  └─────┬─────┘  │
│       └───────────────┼────────────────┘         │
│                   ┌───┴────┐                      │
│                   │ Skill  │                      │
│                   │Registry│                      │
│                   └───┬────┘                      │
│  ┌──────────┐  ┌──────┴───────┐  ┌────────────┐  │
│  │ 内置Skill │  │ WASM 沙箱引擎 │  │ 安全管道   │  │
│  │ (Native) │  │ (wasmtime)   │  │ (Guardrails)│  │
│  └──────────┘  └──────┬───────┘  └────────────┘  │
│                       │                           │
│              ┌────────┴────────┐                  │
│              │  WASM Skill 模块 │                  │
│              │  (第三方开发)    │                  │
│              └─────────────────┘                  │
└─────────────────────────────────────────────────┘
```

---

## 2. Skill 类型

### 2.1 内置 Skill（Native）

| 特性 | 说明 |
|------|------|
| 存放位置 | `src/skill/builtins/` |
| 执行方式 | 原生 Rust 代码 |
| 权限 | 可访问内部 API、LLM 客户端 |
| 适用场景 | 需要深度集成宿主环境的功能 |

**示例**: `code.explainer`、`code.test.generator`

### 2.2 外部 Skill（WASM）

| 特性 | 说明 |
|------|------|
| 存放位置 | `$AGENTHUB_HOME/skills/<name>/` |
| 执行方式 | WASM 沙箱（wasmtime） |
| 权限 | 严格隔离，默认无网络/文件系统访问 |
| 适用场景 | 第三方扩展、用户自定义功能 |

**示例**: `example.hello.world`（见 `examples/skill-hello-world/`）

---

## 3. 开发环境准备

### 3.1 安装 Rust 工具链

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 WASM 编译目标（WASM Skill 必需）
rustup target add wasm32-wasip1

# 安装开发工具
rustup component add rustfmt clippy rust-analyzer
```

### 3.2 验证安装

```bash
rustc --version          # 应 >= 1.75
cargo --version          # 应 >= 1.75
rustup target list       # 应包含 wasm32-wasip1
```

---

## 4. WASM Skill 开发

### 4.1 创建项目

```bash
# 创建库项目
cargo new --lib my-skill
cd my-skill
```

### 4.2 配置 Cargo.toml

```toml
[package]
name = "my-skill"
version = "0.1.0"
edition = "2021"
description = "My custom AgentHub Skill"

# 关键配置：必须包含 "cdylib"
# cdylib = C dynamic library，WASM 模块需要此编译格式
[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 4.3 实现 Skill 逻辑

一个完整的 WASM Skill 包含四个部分：

#### 4.3.1 数据结构定义

```rust
use serde::{Deserialize, Serialize};

/// 输入参数 - 根据 Skill 需求自定义
#[derive(Deserialize)]
struct SkillInput {
    operation: String,
    // ... 其他字段
}

/// 输出结果 - 必须遵循 AgentHub 标准格式
#[derive(Serialize)]
struct SkillOutput {
    output: serde_json::Value,    // 主要输出
    files_created: Vec<String>,   // 创建的文件
    warnings: Vec<String>,        // 警告信息
}
```

#### 4.3.2 FFI 入口函数

```rust
/// Skill 执行入口
/// 
/// 宿主环境通过此函数调用 Skill：
///   - input_ptr: 输入数据在 WASM 内存中的指针
///   - input_len: 输入数据的字节长度
///   - 返回值: 结果数据在 WASM 内存中的指针
#[no_mangle]
pub extern "C" fn execute(input_ptr: i32, input_len: i32) -> i32 {
    // 1. 读取输入
    let input_bytes = unsafe {
        std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize)
    };

    // 2. 解析 JSON
    let input: SkillInput = match serde_json::from_slice(input_bytes) {
        Ok(parsed) => parsed,
        Err(e) => return allocate_error(&format!("JSON 解析失败: {}", e)),
    };

    // 3. 执行逻辑
    let result = execute_operation(&input);

    // 4. 返回结果
    allocate_result(&result)
}
```

#### 4.3.3 业务逻辑

```rust
fn execute_operation(input: &SkillInput) -> SkillOutput {
    match input.operation.as_str() {
        "greet" => SkillOutput {
            output: serde_json::json!({
                "message": "Hello from my skill!"
            }),
            files_created: vec![],
            warnings: vec![],
        },
        unknown => SkillOutput {
            output: serde_json::json!({
                "error": format!("不支持的操作: {}", unknown)
            }),
            files_created: vec![],
            warnings: vec!["未知操作类型".to_string()],
        },
    }
}
```

#### 4.3.4 内存分配

```rust
/// 将结果序列化并分配到 WASM 内存
/// 
/// 内存布局: [4字节长度][4字节填充][N字节JSON数据]
///           ^ ptr        ^ ptr+4      ^ ptr+8
fn allocate_result(result: &SkillOutput) -> i32 {
    let json = serde_json::to_string(result).unwrap();
    allocate_json_to_memory(&json)
}

fn allocate_json_to_memory(json: &str) -> i32 {
    let len = json.len();
    let ptr = unsafe {
        let p = libc::malloc((len + 8) as usize) as i32;
        if p == 0 { return -1; }
        std::ptr::write(p as *mut i32, len as i32);
        std::ptr::copy_nonoverlapping(json.as_ptr(), (p + 8) as *mut u8, len);
        p
    };
    ptr
}
```

### 4.4 编译

```bash
cargo build --target wasm32-wasip1 --release
```

编译产物：`target/wasm32-wasip1/release/my_skill.wasm`

---

## 5. 接口规范详解

### 5.1 函数签名

```rust
#[no_mangle]
pub extern "C" fn execute(input_ptr: i32, input_len: i32) -> i32
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `input_ptr` | `i32` | 输入数据在 WASM 线性内存中的起始地址 |
| `input_len` | `i32` | 输入数据的字节长度 |
| **返回值** | `i32` | 结果数据的指针地址，-1 表示错误 |

### 5.2 内存布局

```
输入/输出内存布局:
┌──────────────┬──────────────┬──────────────────────┐
│ 4 字节: 长度  │ 4 字节: 填充  │ N 字节: JSON 数据     │
│ (ptr)        │ (ptr + 4)    │ (ptr + 8)            │
└──────────────┴──────────────┴──────────────────────┘
```

### 5.3 数据格式

**输入**（由宿主传递）：
```json
{
  "operation": "greet",
  "name": "Alice"
}
```

**输出**（Skill 返回）：
```json
{
  "output": {
    "message": "Hello, Alice! Welcome to AgentHub Skill!"
  },
  "files_created": [],
  "warnings": []
}
```

### 5.4 manifest.yaml

每个 Skill 需要一个 `manifest.yaml` 文件：

```yaml
name: "my.custom.skill"
version: "1.0.0"
description: "My custom skill description"
author: "Your Name"
interfaces: ["cli", "web", "api"]
requires_write: false

input_schema:
  type: object
  properties:
    operation:
      type: string
      description: "Operation type"
  required: ["operation"]
```

---

## 6. 内存管理

### 6.1 分配策略

WASM 使用线性内存模型。Skill 通过 `libc::malloc` 分配内存：

```rust
let ptr = libc::malloc(size) as i32;
```

### 6.2 内存回收

- Skill 分配的内存由宿主环境负责读取和释放
- WASM 模块卸载时，所有线性内存自动回收
- 不需要显式的 `free` 调用

### 6.3 内存限制

默认配置：
- 最大内存：256 MB
- 可通过 `WasmSkillConfig.max_memory_mb` 调整

---

## 7. 安全沙箱

### 7.1 沙箱隔离

WASM Skill 运行在 wasmtime 沙箱中，提供以下隔离：

| 资源 | 默认权限 | 说明 |
|------|----------|------|
| 文件系统 | 禁止 | 无法读写宿主文件系统 |
| 网络 | 禁止 | 无法建立网络连接 |
| 环境变量 | 受限 | 仅 WASI 提供的环境变量可见 |
| 系统调用 | 禁止 | 仅支持 WASI 子集 |

### 7.2 超时控制

默认执行超时：30 秒
可通过 `WasmSkillConfig.max_execution_time_secs` 调整

### 7.3 WASI 支持

Skill 可以继承标准输出和标准错误：

```rust
let wasi = WasiCtxBuilder::new()
    .inherit_stdout()
    .inherit_stderr()
    .build();
```

---

## 8. 测试策略

### 8.1 单元测试

在 Skill 项目中添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet_operation() {
        let input = SkillInput {
            operation: "greet".to_string(),
            name: "Alice".to_string(),
            a: 0.0,
            b: 0.0,
        };
        let result = execute_operation(&input).unwrap();
        // 验证输出...
    }
}
```

### 8.2 集成测试

使用 `tests/test-wasm-skill/` 验证沙箱安全性：

```bash
# 编译测试 Skill
cd tests/test-wasm-skill
cargo build --target wasm32-wasip1 --release

# 在 AgentHub 中加载并测试
agenthub skill run test-wasm-skill '{"test_type": "fs_read"}'
agenthub skill run test-wasm-skill '{"test_type": "network"}'
```

### 8.3 测试检查清单

- [ ] 正常输入返回正确结果
- [ ] 无效 JSON 输入返回错误
- [ ] 未知操作返回错误
- [ ] 边界条件处理正确
- [ ] 沙箱隔离生效（无法访问文件系统/网络）

---

## 9. 发布与部署

### 9.1 发布流程

1. 编译 WASM 模块
2. 创建 Skill 目录结构
3. 放置 `manifest.yaml` 和 `.wasm` 文件
4. 将目录放入 AgentHub 的 skills 目录

### 9.2 目录结构

```
skills/
└── my-skill/
    ├── my_skill.wasm       # 编译后的 WASM 模块
    ├── manifest.yaml       # Skill 元数据
    └── SKILL.md            # 说明文档（可选）
```

### 9.3 安装命令

```bash
# 列出已安装的 Skills
agenthub skill list

# 启用/禁用 Skill
agenthub skill enable my.custom.skill
agenthub skill disable my.custom.skill

# 运行 Skill
agenthub skill run my.custom.skill '{"operation": "greet"}'
```

---

## 10. 最佳实践

### 10.1 代码规范

- 使用 `serde` 的 derive 宏简化序列化
- 错误处理使用 `Result` 类型，不要 panic
- 为公开函数添加文档注释
- 遵循 Rust 命名约定

### 10.2 安全建议

- 验证所有输入参数
- 不依赖未初始化的内存
- 避免无限循环
- 不尝试绕过沙箱限制

### 10.3 性能优化

- 减少内存分配次数
- 避免不必要的 JSON 序列化/反序列化
- 使用 `&str` 而非 `String` 作为函数参数

### 10.4 错误处理

```rust
// 推荐：返回结构化错误
fn execute_operation(input: &SkillInput) -> Result<SkillOutput, String> {
    match input.operation.as_str() {
        "valid" => Ok(SkillOutput { ... }),
        unknown => Err(format!("不支持的操作: {}", unknown)),
    }
}
```

---

## 11. 常见问题

### Q1: 编译提示找不到 `wasm32-wasip1` 目标？

```bash
rustup target add wasm32-wasip1
```

### Q2: 为什么需要 `crate-type = ["cdylib"]`？

WASM 模块需要编译为动态库格式。缺少此配置会编译为 rlib，WASM 加载器无法识别。

### Q3: 可以使用异步代码（async/await）吗？

不可以。WASM Skill 运行在同步执行环境中。需要异步操作时通过宿主接口实现。

### Q4: 如何调试 WASM Skill？

1. 在 Skill 中使用 `eprintln!` 输出调试信息（会显示在宿主 stderr）
2. 使用 `wasmtime` CLI 直接运行 WASM 模块
3. 检查返回的 JSON 格式是否正确

### Q5: Skill 可以访问 LLM 客户端吗？

WASM Skill 无法直接访问 LLM 客户端。需要通过宿主环境提供的接口或内置 Skill 实现。

### Q6: 如何创建文件？

WASM 沙箱默认禁止文件访问。需要宿主环境通过 WASI 配置授权特定目录。

---

## 附录

### A. 参考示例

完整的示例项目位于：
```
examples/skill-hello-world/
```

包含：
- `Cargo.toml` - 项目配置
- `src/lib.rs` - 完整实现（带详细注释）
- `manifest.yaml` - 元数据定义
- `SKILL.md` - 人类可读说明
- `README.md` - 开发者指南

### B. 相关文档

- [项目规范和需求文档](./项目规范和需求文档.md)
- [项目功能文档](./项目功能文档.md)
- [开发实战指南](./开发实战指南.md)

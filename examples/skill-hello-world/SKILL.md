# SKILL.md - Skill Hello World

> AgentHub Skill 开发入门示例

## 概述

`example.hello.world` 是 AgentHub 平台的 Skill 开发入门示例。它演示了如何编写一个可以在 WASM 沙箱中运行的 Skill，包含标准的接口规范、数据格式和内存管理方式。

## 功能

| 操作 | 说明 | 输入参数 | 输出 |
|------|------|----------|------|
| `greet` | 发送个性化问候 | `name`: 用户名 | 问候消息 |
| `echo` | 回显输入内容 | `name`: 回显内容 | 原始内容 |
| `math` | 执行加法运算 | `a`, `b`: 操作数 | 计算结果和公式 |

## 使用方式

### CLI 模式

```bash
# 通过 agenthub 运行
agenthub skill run example.hello.world '{"operation": "greet", "name": "Alice"}'
```

### API 模式

```json
POST /api/skills/execute
{
  "skill_name": "example.hello.world",
  "params": {
    "operation": "greet",
    "name": "Alice"
  }
}
```

## 技术架构

- **运行环境**: WebAssembly (WASM) via wasmtime
- **编程语言**: Rust
- **接口方式**: FFI（外部函数接口）
- **数据格式**: JSON

## 安全说明

- 此 Skill 运行在 WASM 沙箱中
- 默认无网络访问权限
- 默认无文件系统访问权限
- 内存限制：256MB
- 超时限制：30 秒

// =============================================================================
// AgentHub Skill Hello World - 核心实现
// =============================================================================
//
// 本文件是 WASM Skill 的入口点，演示了 AgentHub Skill 的标准接口规范。
//
// 【核心概念】
//
// 1. WASM Skill 通过 FFI（外部函数接口）与宿主环境通信
//    - 宿主通过 `execute(input_ptr, input_len)` 调用 Skill
//    - Skill 通过内存指针读取输入、返回结果
//
// 2. 数据传递方式
//    - 输入：宿主将 JSON 参数写入 WASM 内存，传递指针和长度
//    - 输出：Skill 将 JSON 结果写入 WASM 内存，返回结果指针
//    - 内存布局：[4字节长度][N字节JSON数据]
//
// 3. 沙箱限制
//    - WASM Skill 运行在 wasmtime 沙箱中
//    - 默认无网络访问、无文件系统访问（除非宿主授权）
//    - 内存和超时由宿主配置
//
// 【编译方法】
//   cargo build --target wasm32-wasip1 --release
//
// =============================================================================

use serde::{Deserialize, Serialize};

// =========================================================================
// 第一部分：数据结构定义
// =========================================================================

/// Skill 输入参数
///
/// 宿主环境通过 JSON 传递参数给 Skill，此结构体用于反序列化。
/// 每个 Skill 可以根据自己的需求定义不同的输入结构。
///
/// # 示例输入
/// ```json
/// {
///   "operation": "greet",
///   "name": "World"
/// }
/// ```
#[derive(Deserialize, Debug)]
struct SkillInput {
    /// 操作类型：演示不同的 Skill 行为
    ///
    /// 基础操作:
    /// - "greet": 返回问候语
    /// - "echo":  回显输入
    /// - "math":  简单数学计算
    ///
    /// 沙箱安全测试操作:
    /// - "test_fs_read":  尝试读取系统文件（验证文件隔离）
    /// - "test_fs_write": 尝试写入文件系统（验证写入隔离）
    /// - "test_env":      尝试访问环境变量（验证环境隔离）
    /// - "test_network":  尝试建立网络连接（验证网络隔离）
    operation: String,

    /// 操作参数，根据 operation 不同有不同含义
    #[serde(default)]
    name: String,

    /// 数学计算的两个操作数（仅 math 模式使用）
    #[serde(default)]
    a: f64,

    #[serde(default)]
    b: f64,
}

/// Skill 输出结果
///
/// 这是 AgentHub Skill 的标准输出格式，必须包含以下字段：
/// - output: 主要输出内容（任意 JSON 值）
/// - files_created: 如果 Skill 创建了文件，列出路径
/// - warnings: 警告信息列表
#[derive(Serialize, Debug)]
struct SkillOutput {
    /// 主要输出内容
    output: serde_json::Value,

    /// 创建的文件列表（本示例不创建文件）
    files_created: Vec<String>,

    /// 警告信息（本示例无警告）
    warnings: Vec<String>,
}

// =========================================================================
// 第二部分：FFI 入口函数
// =========================================================================

/// Skill 执行入口函数
///
/// # 函数签名
/// `execute(input_ptr: i32, input_len: i32) -> i32`
///
/// # 参数说明
/// - `input_ptr`: 输入数据在 WASM 内存中的起始地址（指针）
/// - `input_len`: 输入数据的字节长度
///
/// # 返回值
/// - 成功：返回结果数据在 WASM 内存中的指针地址
/// - 失败：返回 -1（负数表示错误）
///
/// # 内存布局
/// 输入和输出都遵循相同的内存布局：
/// ```
/// 地址: [ptr]        [ptr+4]
/// 内容: [4字节长度]  [N字节JSON数据]
/// ```
///
/// # 安全注意
/// 此函数使用 `unsafe` 块访问原始内存指针。
/// 指针由宿主环境提供，我们假设它是有效的。
/// 在生产环境中，应添加更多的边界检查。
///
/// # no_mangle 说明
/// `#[no_mangle]` 阻止 Rust 编译器对函数名进行 name mangling，
/// 确保宿主环境可以通过固定的函数名 "execute" 找到并调用此函数。
#[no_mangle]
pub extern "C" fn execute(input_ptr: i32, input_len: i32) -> i32 {
    // ---------------------------------------------------------------------
    // 步骤 1：从 WASM 内存中读取输入数据
    // ---------------------------------------------------------------------
    // 使用 unsafe 块创建指向输入数据的字节切片。
    // std::slice::from_raw_parts 是零拷贝操作，直接引用内存。
    let input_bytes = unsafe {
        std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize)
    };

    // ---------------------------------------------------------------------
    // 步骤 2：反序列化 JSON 输入
    // ---------------------------------------------------------------------
    let input: SkillInput = match serde_json::from_slice(input_bytes) {
        Ok(parsed) => parsed,
        Err(e) => {
            // 如果 JSON 解析失败，返回错误信息
            return allocate_error(&format!("JSON 解析失败: {}", e));
        }
    };

    // ---------------------------------------------------------------------
    // 步骤 3：执行业务逻辑
    // ---------------------------------------------------------------------
    let result = match execute_operation(&input) {
        Ok(output) => output,
        Err(e) => {
            return allocate_error(&e);
        }
    };

    // ---------------------------------------------------------------------
    // 步骤 4：序列化结果并分配内存
    // ---------------------------------------------------------------------
    allocate_result(&result)
}

// =========================================================================
// 第三部分：业务逻辑实现
// =========================================================================

/// 执行具体的操作逻辑
///
/// 这是一个纯 Rust 函数，不涉及任何 unsafe 操作。
/// 根据输入的操作类型执行不同的逻辑。
///
/// # 支持的操作
/// - "greet": 返回个性化的问候语
/// - "echo":  回显输入的参数
/// - "math":  执行加法运算
fn execute_operation(input: &SkillInput) -> Result<SkillOutput, String> {
    match input.operation.as_str() {
        // -----------------------------------------------------------------
        // 操作 1：问候
        // -----------------------------------------------------------------
        // 示例输入: {"operation": "greet", "name": "Alice"}
        // 示例输出: {"message": "Hello, Alice! Welcome to AgentHub Skill!"}
        "greet" => {
            let name = if input.name.is_empty() {
                "World"
            } else {
                &input.name
            };
            let message = format!("Hello, {}! Welcome to AgentHub Skill!", name);

            Ok(SkillOutput {
                output: serde_json::json!({
                    "message": message,
                    "operation": "greet",
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // -----------------------------------------------------------------
        // 操作 2：回显
        // -----------------------------------------------------------------
        // 示例输入: {"operation": "echo", "name": "test_data"}
        // 示例输出: {"echoed": "test_data", "operation": "echo"}
        "echo" => {
            Ok(SkillOutput {
                output: serde_json::json!({
                    "echoed": input.name,
                    "operation": "echo",
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // -----------------------------------------------------------------
        // 操作 3：数学计算
        // -----------------------------------------------------------------
        // 示例输入: {"operation": "math", "a": 3.5, "b": 2.1}
        // 示例输出: {"result": 5.6, "operation": "math", "formula": "3.5 + 2.1 = 5.6"}
        "math" => {
            let result = input.a + input.b;
            let formula = format!("{} + {} = {}", input.a, input.b, result);

            Ok(SkillOutput {
                output: serde_json::json!({
                    "result": result,
                    "operation": "math",
                    "formula": formula,
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // =================================================================
        // 沙箱安全测试操作
        // =================================================================
        // 以下操作用于验证 WASM 沙箱的隔离能力。
        // 每个操作都会尝试突破沙箱限制，并报告结果。
        // 预期结果：所有操作都应该被沙箱阻止。

        // -----------------------------------------------------------------
        // 测试 1：文件系统读取
        // -----------------------------------------------------------------
        // 尝试读取系统文件，验证沙箱是否阻止文件访问。
        // 预期：沙箱阻止读取，返回错误。
        "test_fs_read" => {
            let result = match std::fs::read_to_string("C:/Windows/System32/drivers/etc/hosts") {
                Ok(content) => {
                    format!("FAIL: 沙箱未生效！成功读取系统文件，内容长度: {} 字节", content.len())
                }
                Err(e) => {
                    format!("PASS: 沙箱生效！无法读取系统文件: {}", e)
                }
            };

            Ok(SkillOutput {
                output: serde_json::json!({
                    "test": "文件系统读取",
                    "result": result,
                    "operation": "test_fs_read",
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // -----------------------------------------------------------------
        // 测试 2：文件系统写入
        // -----------------------------------------------------------------
        // 尝试在系统目录创建文件，验证沙箱是否阻止写入。
        // 预期：沙箱阻止写入，返回错误。
        "test_fs_write" => {
            let result = match std::fs::write("C:/test_sandbox.txt", "sandbox test") {
                Ok(_) => "FAIL: 沙箱未生效！成功写入系统目录".to_string(),
                Err(e) => format!("PASS: 沙箱生效！无法写入系统目录: {}", e),
            };

            Ok(SkillOutput {
                output: serde_json::json!({
                    "test": "文件系统写入",
                    "result": result,
                    "operation": "test_fs_write",
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // -----------------------------------------------------------------
        // 测试 3：环境变量访问
        // -----------------------------------------------------------------
        // 尝试读取环境变量，验证沙箱是否隔离环境。
        // 预期：沙箱限制环境变量可见性。
        "test_env" => {
            let env_vars: Vec<_> = std::env::vars().take(5).collect();
            let result = if env_vars.is_empty() {
                "PASS: 沙箱生效！无法访问环境变量".to_string()
            } else {
                format!("WARN: 部分环境变量可见: {:?}", env_vars)
            };

            Ok(SkillOutput {
                output: serde_json::json!({
                    "test": "环境变量访问",
                    "result": result,
                    "operation": "test_env",
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // -----------------------------------------------------------------
        // 测试 4：网络连接
        // -----------------------------------------------------------------
        // 尝试建立 TCP 连接，验证沙箱是否阻止网络访问。
        // 预期：沙箱阻止网络连接。
        "test_network" => {
            let result = match std::net::TcpStream::connect("127.0.0.1:80") {
                Ok(_) => "FAIL: 沙箱未生效！可以建立网络连接".to_string(),
                Err(e) => format!("PASS: 沙箱生效！无法建立网络连接: {}", e),
            };

            Ok(SkillOutput {
                output: serde_json::json!({
                    "test": "网络连接",
                    "result": result,
                    "operation": "test_network",
                }),
                files_created: vec![],
                warnings: vec![],
            })
        }

        // -----------------------------------------------------------------
        // 未知操作
        // -----------------------------------------------------------------
        unknown => {
            Err(format!(
                "不支持的操作: '{}'. 支持的操作: greet, echo, math, test_fs_read, test_fs_write, test_env, test_network",
                unknown
            ))
        }
    }
}

// =========================================================================
// 第四部分：内存分配与结果编码
// =========================================================================

/// 将成功结果序列化并分配到 WASM 内存
///
/// # 内存布局
/// ```
/// [4 字节: JSON 长度] [N 字节: JSON 数据]
///  ^ result_ptr        ^ result_ptr + 8
/// ```
///
/// # 返回值
/// 返回结果数据在 WASM 内存中的指针地址。
/// 宿主环境将通过此指针读取 Skill 的输出。
fn allocate_result(result: &SkillOutput) -> i32 {
    let json_string = match serde_json::to_string(result) {
        Ok(s) => s,
        Err(e) => {
            // 如果序列化失败，返回错误信息
            return allocate_error(&format!("结果序列化失败: {}", e));
        }
    };

    allocate_json_to_memory(&json_string)
}

/// 将错误信息包装为标准格式并分配到 WASM 内存
///
/// 错误结果也遵循 SkillOutput 格式，但 output 中包含错误信息。
fn allocate_error(error_message: &str) -> i32 {
    let error_result = SkillOutput {
        output: serde_json::json!({
            "error": error_message,
        }),
        files_created: vec![],
        warnings: vec!["Skill 执行过程中发生错误".to_string()],
    };

    let json_string = serde_json::to_string(&error_result).unwrap_or_else(|_| {
        // 如果连错误结果都无法序列化，返回最小化的错误 JSON
        r#"{"output":{"error":"未知错误"},"files_created":[],"warnings":[]}"#.to_string()
    });

    allocate_json_to_memory(&json_string)
}

/// 将 JSON 字符串分配到 WASM 内存中
///
/// # 内存布局
/// ```
/// 地址:    [ptr]              [ptr + 4]          [ptr + 8]
/// 内容:    [4字节: 长度]      [4字节: 填充]      [N字节: JSON数据]
/// 大小:    4 bytes            4 bytes            N bytes
/// ```
///
/// # 为什么需要 8 字节头部？
/// - 前 4 字节存储数据长度，方便宿主读取
/// - 中间 4 字节作为填充（padding），确保数据对齐
/// - 实际数据从 ptr + 8 开始
///
/// # 内存分配
/// 使用 libc::malloc 分配内存。这块内存由宿主环境负责释放。
/// 在 WASM 中，内存是线性的，分配的内存在模块卸载时自动回收。
///
/// # 返回值
/// - 成功：返回分配的内存指针地址
/// - 失败：返回 -1
fn allocate_json_to_memory(json: &str) -> i32 {
    let len = json.len();

    // 使用 libc::malloc 分配内存
    // 需要 len + 8 字节：4 字节长度 + 4 字节填充 + N 字节数据
    let result_ptr = unsafe {
        let ptr = libc::malloc((len + 8) as usize) as i32;

        // 检查内存分配是否成功
        if ptr == 0 {
            return -1; // 内存分配失败
        }

        // 写入长度信息（前 4 字节）
        std::ptr::write(ptr as *mut i32, len as i32);

        // 写入 JSON 数据（从 ptr + 8 开始）
        std::ptr::copy_nonoverlapping(
            json.as_ptr(),
            (ptr + 8) as *mut u8,
            len,
        );

        ptr
    };

    result_ptr
}

// =========================================================================
// 第五部分：辅助函数（可选，用于调试）
// =========================================================================

/// 获取 Skill 的版本信息
///
/// 此函数不是必需的，但可以用于宿主环境查询 Skill 版本。
/// 如果需要，可以添加 `#[no_mangle]` 使其可从外部调用。
#[allow(dead_code)]
fn skill_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 获取 Skill 的名称
#[allow(dead_code)]
fn skill_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub const CODE_EXPLAINER_TEMPLATE: &str = r#"
You are a helpful code explanation assistant. Please explain the following code in a clear and concise manner.

Language: {{language}}
File: {{file_path}}

Code:
```{{language}}
{{code}}
```

Please provide:
1. A high-level summary of what this code does
2. Key components and their responsibilities
3. Any potential issues, edge cases, or improvements
4. Time and space complexity if applicable
"#;

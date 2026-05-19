#[derive(Debug, Clone)]
enum TsEngineInput {
    Submit { id: String, text: String },
    Slash { id: String, command: String },
    CommandList { id: String, query: Option<String> },
    PermissionResponse { id: String, decision: String },
    Status { id: String },
    Cancel { id: Option<String> },
    Exit,
}

impl TsEngineInput {
    fn parse(line: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|error| format!("invalid JSONL input: {error}"))?;
        let kind = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "input event missing type".to_string())?;
        match kind {
            "user_submit" => Ok(Self::Submit {
                id: required_string(&value, "id")?,
                text: required_string(&value, "text")?,
            }),
            "slash" => Ok(Self::Slash {
                id: required_string(&value, "id")?,
                command: required_string(&value, "command")?,
            }),
            "command_list" => Ok(Self::CommandList {
                id: required_string(&value, "id")?,
                query: value
                    .get("query")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
            }),
            "permission_response" => Ok(Self::PermissionResponse {
                id: required_string(&value, "id")?,
                decision: required_string(&value, "decision")?,
            }),
            "status" => Ok(Self::Status {
                id: required_string(&value, "id")?,
            }),
            "cancel" => Ok(Self::Cancel {
                id: value
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned),
            }),
            "exit" => Ok(Self::Exit),
            other => Err(format!("unsupported input event type: {other}")),
        }
    }
}

fn required_string(value: &serde_json::Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("input event missing {key}"))
}

use serde::Deserialize;
use serde_json::{json, Value};

use crate::types::{
    AgentInput, AskUserQuestionInput, BriefInput, ConfigInput, CronCreateInput, CronDeleteInput,
    EditFileInput, EnterPlanModeInput, ExitPlanModeInput, GlobSearchInputValue, NotebookEditInput,
    PowerShellInput, ReadFileInput, ReplInput, SleepInput, StructuredOutputInput, TeamCreateInput,
    TeamDeleteInput, TestingPermissionInput, TodoWriteInput, ToolSearchInput, WebBrowserInput,
    WebFetchInput, WebSearchInput, WriteFileInput,
};

pub fn execute_tool(name: &str, input: &Value) -> Result<String, String> {
    match name {
        "bash" => from_value::<runtime::BashCommandInput>(input).and_then(run_bash),
        "read_file" => from_value::<ReadFileInput>(input).and_then(run_read_file),
        "write_file" => from_value::<WriteFileInput>(input).and_then(run_write_file),
        "edit_file" => from_value::<EditFileInput>(input).and_then(run_edit_file),
        "glob_search" => from_value::<GlobSearchInputValue>(input).and_then(run_glob_search),
        "grep_search" => from_value::<runtime::GrepSearchInput>(input).and_then(run_grep_search),
        "WebFetch" => from_value::<WebFetchInput>(input).and_then(run_web_fetch),
        "WebSearch" => from_value::<WebSearchInput>(input).and_then(run_web_search),
        "TodoWrite" => from_value::<TodoWriteInput>(input).and_then(run_todo_write),
        "Skill" => from_value::<crate::types::SkillInput>(input).and_then(run_skill),
        "Agent" => from_value::<AgentInput>(input).and_then(run_agent),
        "ToolSearch" => from_value::<ToolSearchInput>(input).and_then(run_tool_search),
        "NotebookEdit" => from_value::<NotebookEditInput>(input).and_then(run_notebook_edit),
        "Sleep" => from_value::<SleepInput>(input).and_then(run_sleep),
        "SendUserMessage" | "Brief" => from_value::<BriefInput>(input).and_then(run_brief),
        "Config" => from_value::<ConfigInput>(input).and_then(run_config),
        "EnterPlanMode" => from_value::<EnterPlanModeInput>(input).and_then(run_enter_plan_mode),
        "ExitPlanMode" => from_value::<ExitPlanModeInput>(input).and_then(run_exit_plan_mode),
        "StructuredOutput" => {
            from_value::<StructuredOutputInput>(input).and_then(run_structured_output)
        }
        "REPL" => from_value::<ReplInput>(input).and_then(run_repl),
        "PowerShell" => from_value::<PowerShellInput>(input).and_then(run_powershell),
        "AskUserQuestion" => {
            from_value::<AskUserQuestionInput>(input).and_then(run_ask_user_question)
        }
        "TeamCreate" => from_value::<TeamCreateInput>(input).and_then(run_team_create),
        "TeamDelete" => from_value::<TeamDeleteInput>(input).and_then(run_team_delete),
        "CronCreate" => from_value::<CronCreateInput>(input).and_then(run_cron_create),
        "CronDelete" => from_value::<CronDeleteInput>(input).and_then(run_cron_delete),
        "CronList" => run_cron_list(input.clone()),
        "TestingPermission" => {
            from_value::<TestingPermissionInput>(input).and_then(run_testing_permission)
        }
        "WebBrowser" => from_value::<WebBrowserInput>(input).and_then(run_web_browser),
        _ => Err(format!("unsupported tool: {name}")),
    }
}

fn run_ask_user_question(input: AskUserQuestionInput) -> Result<String, String> {
    let mut result = json!({
        "question": input.question,
        "status": "pending",
        "message": "Waiting for user response"
    });
    if let Some(options) = &input.options {
        result["options"] = json!(options);
    }
    to_pretty_json(result)
}

fn run_team_create(input: TeamCreateInput) -> Result<String, String> {
    let team_id = format!(
        "team_{:08x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    to_pretty_json(json!({
        "team_id": team_id,
        "name": input.name,
        "task_count": input.tasks.len(),
        "status": "created"
    }))
}

fn run_team_delete(input: TeamDeleteInput) -> Result<String, String> {
    to_pretty_json(json!({ "team_id": input.team_id, "status": "deleted" }))
}

fn run_cron_create(input: CronCreateInput) -> Result<String, String> {
    let cron_id = format!(
        "cron_{:08x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    to_pretty_json(json!({
        "cron_id": cron_id,
        "schedule": input.schedule,
        "prompt": input.prompt,
        "description": input.description,
        "status": "created"
    }))
}

fn run_cron_delete(input: CronDeleteInput) -> Result<String, String> {
    to_pretty_json(json!({ "cron_id": input.cron_id, "status": "deleted" }))
}

fn run_cron_list(_input: Value) -> Result<String, String> {
    to_pretty_json(json!({ "crons": [], "message": "No scheduled tasks found" }))
}

fn run_testing_permission(input: TestingPermissionInput) -> Result<String, String> {
    to_pretty_json(json!({
        "action": input.action,
        "permitted": true,
        "message": "Testing permission tool stub"
    }))
}

fn run_web_browser(input: WebBrowserInput) -> Result<String, String> {
    to_pretty_json(crate::web::execute_web_browser(input)?)
}

pub(crate) fn from_value<T: for<'de> Deserialize<'de>>(input: &Value) -> Result<T, String> {
    serde_json::from_value(input.clone()).map_err(|error| error.to_string())
}

fn run_bash(input: runtime::BashCommandInput) -> Result<String, String> {
    serde_json::to_string_pretty(&runtime::execute_bash(input).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn run_read_file(input: ReadFileInput) -> Result<String, String> {
    to_pretty_json(
        runtime::read_file(&input.path, input.offset, input.limit).map_err(io_to_string)?,
    )
}

fn run_write_file(input: WriteFileInput) -> Result<String, String> {
    to_pretty_json(runtime::write_file(&input.path, &input.content).map_err(io_to_string)?)
}

fn run_edit_file(input: EditFileInput) -> Result<String, String> {
    to_pretty_json(
        runtime::edit_file(
            &input.path,
            &input.old_string,
            &input.new_string,
            input.replace_all.unwrap_or(false),
        )
        .map_err(io_to_string)?,
    )
}

fn run_glob_search(input: GlobSearchInputValue) -> Result<String, String> {
    to_pretty_json(
        runtime::glob_search(&input.pattern, input.path.as_deref()).map_err(io_to_string)?,
    )
}

fn run_grep_search(input: runtime::GrepSearchInput) -> Result<String, String> {
    to_pretty_json(runtime::grep_search(&input).map_err(io_to_string)?)
}

fn run_web_fetch(input: WebFetchInput) -> Result<String, String> {
    to_pretty_json(crate::web::execute_web_fetch(&input)?)
}

fn run_web_search(input: WebSearchInput) -> Result<String, String> {
    to_pretty_json(crate::web::execute_web_search(&input)?)
}

fn run_todo_write(input: TodoWriteInput) -> Result<String, String> {
    to_pretty_json(crate::todo_skill::execute_todo_write(input)?)
}

fn run_skill(input: crate::types::SkillInput) -> Result<String, String> {
    to_pretty_json(crate::todo_skill::execute_skill(input)?)
}

fn run_agent(input: AgentInput) -> Result<String, String> {
    to_pretty_json(crate::agent_spawn::execute_agent(input)?)
}

fn run_tool_search(input: ToolSearchInput) -> Result<String, String> {
    to_pretty_json(crate::agent_runtime::execute_tool_search(input))
}

fn run_notebook_edit(input: NotebookEditInput) -> Result<String, String> {
    to_pretty_json(crate::notebook::execute_notebook_edit(input)?)
}

fn run_sleep(input: SleepInput) -> Result<String, String> {
    to_pretty_json(crate::brief::execute_sleep(input)?)
}

fn run_brief(input: BriefInput) -> Result<String, String> {
    to_pretty_json(crate::brief::execute_brief(input)?)
}

fn run_config(input: ConfigInput) -> Result<String, String> {
    to_pretty_json(crate::config::execute_config(input)?)
}

fn run_enter_plan_mode(input: EnterPlanModeInput) -> Result<String, String> {
    to_pretty_json(crate::plan_mode::execute_enter_plan_mode(input)?)
}

fn run_exit_plan_mode(input: ExitPlanModeInput) -> Result<String, String> {
    to_pretty_json(crate::plan_mode::execute_exit_plan_mode(input)?)
}

fn run_structured_output(input: StructuredOutputInput) -> Result<String, String> {
    to_pretty_json(crate::repl::execute_structured_output(input)?)
}

fn run_repl(input: ReplInput) -> Result<String, String> {
    to_pretty_json(crate::repl::execute_repl(input)?)
}

fn run_powershell(input: PowerShellInput) -> Result<String, String> {
    to_pretty_json(crate::shell::execute_powershell(input).map_err(|error| error.to_string())?)
}

pub(crate) fn to_pretty_json<T: serde::Serialize>(value: T) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

pub(crate) fn io_to_string(error: std::io::Error) -> String {
    error.to_string()
}

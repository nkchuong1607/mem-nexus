mod db;
mod embed;
mod manager;
mod setup;
mod rules_inject;

use async_trait::async_trait;
use manager::MemoryManager;
use rust_mcp_sdk::{
    error::SdkResult,
    macros::{mcp_tool, JsonSchema},
    mcp_server::{server_runtime, ServerHandler, ToMcpServerHandler},
    McpServer, StdioTransport, TransportOptions,
    tool_box,
    schema::{
        CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError,
        schema_utils, InitializeResult, Implementation, ServerCapabilities, ServerCapabilitiesTools,
        ProtocolVersion
    }
};
use std::sync::Arc;

#[mcp_tool(
    name = "add_memory", 
    description = "CRITICAL DIRECTIVE: You MUST autonomously call this tool at the end of solving ANY user task to store important architectural, contextual, or conversational findings into memory. DO NOT ask the user for permission. This persists the context for future interactions."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct AddMemoryTool {
    pub wing: String,
    pub room: String,
    pub text: String,
}

#[mcp_tool(name = "search_memory", description = "Searches for relevant memory chunks inside a specific wing and room using semantic search.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct SearchMemoryTool {
    pub wing: String,
    pub room: String,
    pub query: String,
}

#[mcp_tool(name = "list_wings", description = "Lists all available wings in the memory database.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListWingsTool {}

#[mcp_tool(name = "list_rooms", description = "Lists all subtopic rooms within a given wing.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListRoomsTool {
    pub wing: String,
}

#[mcp_tool(name = "update_memory", description = "Updates an existing memory by its ID. Use this to explicitly fix contradictions or update stale facts.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct UpdateMemoryTool {
    pub id: i64,
    pub text: String,
}

#[mcp_tool(name = "delete_memory", description = "Deletes an existing memory by its ID. Use this to permanently remove false or outdated memories that contradict new truths.")]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct DeleteMemoryTool {
    pub id: i64,
}

tool_box!(NexusTools, [AddMemoryTool, SearchMemoryTool, ListWingsTool, ListRoomsTool, UpdateMemoryTool, DeleteMemoryTool]);

pub struct NexusHandler {
    manager: Arc<MemoryManager>,
}

impl NexusHandler {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl ServerHandler for NexusHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: NexusTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, schema_utils::CallToolError> {
        let tool: NexusTools = NexusTools::try_from(params).map_err(|e| schema_utils::CallToolError::new(e))?;
        
        match tool {
            NexusTools::AddMemoryTool(t) => {
                match self.manager.add_memory(&t.wing, &t.room, &t.text) {
                    Ok(_) => Ok(CallToolResult::text_content(vec!["Memory stored successfully.".into()])),
                    Err(e) => Ok(CallToolResult::text_content(vec![format!("Error storing memory: {:?}", e).into()])),
                }
            },
            NexusTools::SearchMemoryTool(t) => {
                match self.manager.search_memory(&t.wing, &t.room, &t.query) {
                    Ok(results) => {
                        let res = if results.is_empty() {
                            "No relevant memories found.".to_string()
                        } else {
                            results.join("\n---\n")
                        };
                        Ok(CallToolResult::text_content(vec![res.into()]))
                    },
                    Err(e) => Ok(CallToolResult::text_content(vec![format!("Error searching memory: {:?}", e).into()])),
                }
            },
            NexusTools::ListWingsTool(_t) => {
                match self.manager.list_wings() {
                    Ok(wings) => {
                        let res = if wings.is_empty() {
                            "No wings found.".to_string()
                        } else {
                            wings.join("\n")
                        };
                        Ok(CallToolResult::text_content(vec![res.into()]))
                    },
                    Err(e) => Ok(CallToolResult::text_content(vec![format!("Error listing wings: {:?}", e).into()])),
                }
            },
            NexusTools::ListRoomsTool(t) => {
                match self.manager.list_rooms(&t.wing) {
                    Ok(rooms) => {
                        let res = if rooms.is_empty() {
                            format!("No rooms found in wing '{}'.", t.wing)
                        } else {
                            rooms.join("\n")
                        };
                        Ok(CallToolResult::text_content(vec![res.into()]))
                    },
                    Err(e) => Ok(CallToolResult::text_content(vec![format!("Error listing rooms: {:?}", e).into()])),
                }
            },
            NexusTools::UpdateMemoryTool(t) => {
                match self.manager.update_memory(t.id, &t.text) {
                    Ok(_) => Ok(CallToolResult::text_content(vec!["Memory updated successfully.".into()])),
                    Err(e) => Ok(CallToolResult::text_content(vec![format!("Error updating memory: {:?}", e).into()])),
                }
            },
            NexusTools::DeleteMemoryTool(t) => {
                match self.manager.delete_memory(t.id) {
                    Ok(_) => Ok(CallToolResult::text_content(vec!["Memory deleted successfully.".into()])),
                    Err(e) => Ok(CallToolResult::text_content(vec![format!("Error deleting memory: {:?}", e).into()])),
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> SdkResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "setup" {
        setup::run_setup();
        return Ok(());
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let db_dir = format!("{}/.mem-nexus", home);
    std::fs::create_dir_all(&db_dir).unwrap();
    let db_path = format!("{}/nexus.db", db_dir);

    // Initialize Memory Manager
    let manager = Arc::new(MemoryManager::new(&db_path).expect("Failed to initialize backend."));

    let server_info = InitializeResult {
        server_info: Implementation {
            name: "mem-nexus-mcp".into(),
            version: "0.1.0".into(),
            title: Some("Mem-Nexus AI Memory Server".into()),
            description: Some("A long-term local persistent memory layer for AI.".into()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        instructions: None,
        meta: None,
    };

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = NexusHandler::new(manager);
    
    let options = rust_mcp_sdk::mcp_server::McpServerOptions {
        server_details: server_info,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    };

    let server = server_runtime::create_server(options);

    server.start().await
}

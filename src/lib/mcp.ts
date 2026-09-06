/** The one line that connects Claude Code to darric's MCP server at `url`. */
export function claudeMcpAddCommand(url: string): string {
  return `claude mcp add --transport http darric ${url}`;
}

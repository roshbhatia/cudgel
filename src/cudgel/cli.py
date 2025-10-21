"""Command-line interface for cudgel."""

import asyncio
import json
from pathlib import Path

import click
from rich.console import Console
from rich.table import Table
from rich.tree import Tree
from rich.syntax import Syntax

from cudgel.config import get_config
from cudgel.graph import GraphQuery
from cudgel.indexer import CodeIndexer
from cudgel.query import CodeQuery

console = Console()


@click.group()
@click.version_option(version="0.1.0")
def cli() -> None:
    """Cudgel - Code indexing tool with tree-sitter, Temporal, and PostgreSQL/pgvector."""
    pass


@cli.command()
@click.argument("path", type=click.Path(exists=True, file_okay=False, dir_okay=True))
@click.option("--name", help="Repository name (defaults to directory name)")
def index(path: str, name: str | None) -> None:
    """Index a code repository."""
    console.print(f"[bold blue]Indexing repository:[/bold blue] {path}")

    config = get_config()
    indexer = CodeIndexer(config)

    async def run_index() -> None:
        try:
            await indexer.initialize()
            repo_id = await indexer.index_repository(Path(path), name)
            console.print(f"[bold green]Successfully indexed repository with ID:[/bold green] {repo_id}")
        except Exception as e:
            console.print(f"[bold red]Error:[/bold red] {e}")
            raise
        finally:
            await indexer.close()

    asyncio.run(run_index())


@cli.command()
@click.argument("query_text")
@click.option("--repo", help="Filter by repository path")
@click.option("--limit", default=10, help="Maximum number of results")
@click.option("--type", "search_type", type=click.Choice(["symbols", "code", "both"]), default="both")
@click.option("--json-output", is_flag=True, help="Output as JSON")
def query(query_text: str, repo: str | None, limit: int, search_type: str, json_output: bool) -> None:
    """Query code using natural language."""
    config = get_config()
    query_engine = CodeQuery(config)

    async def run_query() -> None:
        try:
            await query_engine.initialize()
            results = await query_engine.search(
                query_text,
                limit=limit,
                repository_path=repo,
                search_type=search_type,
            )

            if json_output:
                # Convert to JSON-serializable format
                output = {}
                if "symbols" in results:
                    output["symbols"] = [dict(r) for r in results["symbols"]]
                if "code" in results:
                    output["code"] = [dict(r) for r in results["code"]]
                console.print(json.dumps(output, indent=2))
            else:
                _display_query_results(results)

        except Exception as e:
            console.print(f"[bold red]Error:[/bold red] {e}")
            raise
        finally:
            await query_engine.close()

    asyncio.run(run_query())


@cli.command()
@click.argument("symbol_name")
@click.option("--repo", help="Filter by repository path")
@click.option("--depth", default=1, help="Traversal depth for references")
@click.option("--type", "graph_type", type=click.Choice(["references", "calls"]), default="references")
@click.option("--direction", type=click.Choice(["incoming", "outgoing", "both"]), default="both")
@click.option("--json-output", is_flag=True, help="Output as JSON")
def graph(
    symbol_name: str,
    repo: str | None,
    depth: int,
    graph_type: str,
    direction: str,
    json_output: bool
) -> None:
    """Show graph relationships for a symbol."""
    config = get_config()
    graph_query = GraphQuery(config)

    async def run_graph() -> None:
        try:
            await graph_query.initialize()

            if graph_type == "calls":
                result = await graph_query.get_call_graph(symbol_name, repo, direction)
            else:
                result = await graph_query.get_references(symbol_name, repo, depth)

            if json_output:
                console.print(json.dumps(result, indent=2))
            else:
                _display_graph(result, graph_type)

        except Exception as e:
            console.print(f"[bold red]Error:[/bold red] {e}")
            raise
        finally:
            await graph_query.close()

    asyncio.run(run_graph())


@cli.command()
@click.option("--host", default="localhost", help="LSP server host")
@click.option("--port", default=6010, help="LSP server port")
def lsp(host: str, port: int) -> None:
    """Start the LSP server."""
    console.print(f"[bold blue]Starting LSP server on {host}:{port}[/bold blue]")

    from cudgel.lsp_server import start_lsp_server

    try:
        start_lsp_server(host, port)
    except KeyboardInterrupt:
        console.print("\n[bold yellow]LSP server stopped[/bold yellow]")


@cli.command()
def init_db() -> None:
    """Initialize the database schema."""
    console.print("[bold blue]Initializing database schema...[/bold blue]")

    config = get_config()

    async def run_init() -> None:
        from cudgel.database import Database
        db = Database(config)
        try:
            await db.connect()
            await db.init_schema()
            console.print("[bold green]Database schema initialized successfully[/bold green]")
        except Exception as e:
            console.print(f"[bold red]Error:[/bold red] {e}")
            raise
        finally:
            await db.close()

    asyncio.run(run_init())


def _display_query_results(results: dict) -> None:
    """Display query results in a nice format."""
    if "symbols" in results:
        symbols = results["symbols"]
        if symbols:
            console.print("\n[bold cyan]Symbols:[/bold cyan]")
            table = Table(show_header=True)
            table.add_column("Name", style="cyan")
            table.add_column("Kind", style="magenta")
            table.add_column("File", style="green")
            table.add_column("Line", style="yellow")
            table.add_column("Similarity", style="blue")

            for sym in symbols:
                table.add_row(
                    sym["name"],
                    sym["kind"],
                    sym["path"],
                    str(sym["start_line"]),
                    f"{sym['similarity']:.3f}",
                )

            console.print(table)

    if "code" in results:
        code_chunks = results["code"]
        if code_chunks:
            console.print("\n[bold cyan]Code Chunks:[/bold cyan]")
            for i, chunk in enumerate(code_chunks, 1):
                console.print(f"\n[bold]{i}. {chunk['path']}:{chunk['start_line']}[/bold] (similarity: {chunk['similarity']:.3f})")
                if chunk["symbol_name"]:
                    console.print(f"   Symbol: {chunk['symbol_name']} ({chunk['symbol_kind']})")

                # Display code with syntax highlighting
                syntax = Syntax(
                    chunk["text"][:500],  # Limit display length
                    chunk.get("language", "text"),
                    theme="monokai",
                    line_numbers=True,
                    start_line=chunk["start_line"],
                )
                console.print(syntax)


def _display_graph(graph: dict, graph_type: str) -> None:
    """Display graph in a nice format."""
    if "error" in graph:
        console.print(f"[bold red]Error:[/bold red] {graph['error']}")
        return

    console.print(f"\n[bold cyan]Graph for: {graph['root']}[/bold cyan]")
    console.print(f"Nodes: {len(graph['nodes'])}, Edges: {len(graph['edges'])}\n")

    # Create a tree visualization
    tree = Tree(f"[bold]{graph['root']}[/bold]")
    node_map = {n["id"]: n for n in graph["nodes"]}
    root_node = next((n for n in graph["nodes"] if n["name"] == graph["root"]), None)

    if root_node:
        _build_tree(tree, root_node["id"], graph["edges"], node_map, set())
        console.print(tree)

    # Also show as table
    if graph["edges"]:
        console.print("\n[bold cyan]Edges:[/bold cyan]")
        table = Table(show_header=True)
        table.add_column("From", style="cyan")
        table.add_column("To", style="magenta")
        table.add_column("Type", style="yellow")

        for edge in graph["edges"]:
            from_node = node_map.get(edge["from"], {})
            to_node = node_map.get(edge["to"], {})
            table.add_row(
                from_node.get("name", "?"),
                to_node.get("name", "?"),
                edge["type"],
            )

        console.print(table)


def _build_tree(tree: Tree, node_id: int, edges: list, node_map: dict, visited: set) -> None:
    """Recursively build tree visualization."""
    if node_id in visited:
        return

    visited.add(node_id)

    # Find outgoing edges
    for edge in edges:
        if edge["from"] == node_id:
            to_node = node_map.get(edge["to"])
            if to_node:
                label = f"{to_node['name']} ({to_node['kind']}) - {to_node.get('file', 'unknown')}"
                branch = tree.add(label)
                _build_tree(branch, edge["to"], edges, node_map, visited)


def main() -> None:
    """Main entry point."""
    cli()


if __name__ == "__main__":
    main()

"""Basic usage examples for cudgel."""

import asyncio
from pathlib import Path

from cudgel.config import get_config
from cudgel.indexer import CodeIndexer
from cudgel.query import CodeQuery
from cudgel.graph import GraphQuery


async def example_index_repository():
    """Example: Index a repository."""
    print("=== Indexing Repository ===")

    config = get_config()
    indexer = CodeIndexer(config)

    try:
        await indexer.initialize()

        # Index current directory
        repo_path = Path(".")
        repo_id = await indexer.index_repository(repo_path)

        print(f"Successfully indexed repository with ID: {repo_id}")

    finally:
        await indexer.close()


async def example_query_code():
    """Example: Query code with natural language."""
    print("\n=== Querying Code ===")

    config = get_config()
    query_engine = CodeQuery(config)

    try:
        await query_engine.initialize()

        # Search for symbols
        results = await query_engine.search_symbols(
            "function that parses code",
            limit=5
        )

        print(f"\nFound {len(results)} matching symbols:")
        for result in results:
            print(f"  - {result['name']} ({result['kind']}) in {result['path']}:{result['start_line']}")
            print(f"    Similarity: {result['similarity']:.3f}")

        # Search for code chunks
        code_results = await query_engine.search_code(
            "database connection",
            limit=5
        )

        print(f"\nFound {len(code_results)} matching code chunks:")
        for result in code_results:
            print(f"  - {result['path']}:{result['start_line']}-{result['end_line']}")
            if result['symbol_name']:
                print(f"    Symbol: {result['symbol_name']} ({result['symbol_kind']})")
            print(f"    Similarity: {result['similarity']:.3f}")

    finally:
        await query_engine.close()


async def example_graph_relationships():
    """Example: Explore graph relationships."""
    print("\n=== Graph Relationships ===")

    config = get_config()
    graph = GraphQuery(config)

    try:
        await graph.initialize()

        # Get call graph for a function
        symbol_name = "main"  # Change to a function in your codebase
        call_graph = await graph.get_call_graph(
            symbol_name,
            direction="both"
        )

        if "error" in call_graph:
            print(f"Error: {call_graph['error']}")
        else:
            print(f"\nCall graph for '{symbol_name}':")
            print(f"  Nodes: {len(call_graph['nodes'])}")
            print(f"  Edges: {len(call_graph['edges'])}")

            for edge in call_graph['edges'][:10]:  # Show first 10 edges
                from_node = next(n for n in call_graph['nodes'] if n['id'] == edge['from'])
                to_node = next(n for n in call_graph['nodes'] if n['id'] == edge['to'])
                print(f"  {from_node['name']} -> {to_node['name']} ({edge['type']})")

        # Get references
        references = await graph.get_references(
            symbol_name,
            depth=2
        )

        if "error" not in references:
            print(f"\nReferences for '{symbol_name}':")
            print(f"  Nodes: {len(references['nodes'])}")
            print(f"  Edges: {len(references['edges'])}")

    finally:
        await graph.close()


async def example_combined_workflow():
    """Example: Combined workflow - index, query, and explore."""
    print("\n=== Combined Workflow ===")

    config = get_config()

    # 1. Index a small repository
    print("\n1. Indexing repository...")
    indexer = CodeIndexer(config)
    await indexer.initialize()
    repo_id = await indexer.index_repository(Path("."), "cudgel")
    await indexer.close()
    print(f"   Repository indexed with ID: {repo_id}")

    # 2. Query for specific functionality
    print("\n2. Searching for code...")
    query_engine = CodeQuery(config)
    await query_engine.initialize()

    results = await query_engine.search(
        "code that handles database queries",
        limit=3,
        search_type="both"
    )

    if "symbols" in results and results["symbols"]:
        print(f"   Found {len(results['symbols'])} symbols")
        top_symbol = results['symbols'][0]
        print(f"   Top match: {top_symbol['name']} in {top_symbol['path']}")

        # 3. Explore relationships of top match
        print("\n3. Exploring relationships...")
        graph = GraphQuery(config)
        await graph.initialize()

        relationships = await graph.get_references(
            top_symbol['name'],
            depth=1
        )

        if "error" not in relationships:
            print(f"   Found {len(relationships['nodes'])} related symbols")

        await graph.close()

    await query_engine.close()


async def main():
    """Run all examples."""
    print("Cudgel Usage Examples")
    print("=" * 50)

    # Note: Make sure you have initialized the database first:
    # cudgel init-db

    try:
        await example_index_repository()
        await example_query_code()
        await example_graph_relationships()
        # await example_combined_workflow()  # Uncomment to run combined workflow

    except Exception as e:
        print(f"\nError: {e}")
        print("\nMake sure you have:")
        print("1. Started PostgreSQL (make docker-up)")
        print("2. Initialized the database (cudgel init-db)")
        print("3. Set up .env file with correct credentials")


if __name__ == "__main__":
    asyncio.run(main())

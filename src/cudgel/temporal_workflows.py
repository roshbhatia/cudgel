"""Temporal workflows for scheduled code indexing."""

import asyncio
from datetime import timedelta
from pathlib import Path
from typing import Optional

from temporalio import activity, workflow
from temporalio.client import Client
from temporalio.worker import Worker

from cudgel.config import CudgelConfig, get_config
from cudgel.indexer import CodeIndexer


@activity.defn
async def index_repository_activity(repo_path: str, name: Optional[str] = None) -> int:
    """Activity to index a repository."""
    config = get_config()
    indexer = CodeIndexer(config)

    try:
        await indexer.initialize()
        repo_id = await indexer.index_repository(Path(repo_path), name)
        return repo_id
    finally:
        await indexer.close()


@activity.defn
async def check_repository_updates(repo_path: str) -> bool:
    """Activity to check if repository has updates."""
    # In a real implementation, this would check git status, file modifications, etc.
    # For now, we'll just return True to trigger re-indexing
    return True


@workflow.defn
class RepositoryIndexingWorkflow:
    """Workflow for indexing a repository."""

    @workflow.run
    async def run(self, repo_path: str, name: Optional[str] = None) -> int:
        """Run the repository indexing workflow."""
        # Execute the indexing activity
        repo_id = await workflow.execute_activity(
            index_repository_activity,
            args=[repo_path, name],
            start_to_close_timeout=timedelta(hours=1),
        )

        return repo_id


@workflow.defn
class ScheduledIndexingWorkflow:
    """Workflow for scheduled periodic indexing."""

    @workflow.run
    async def run(self, repo_path: str, interval_hours: int = 24) -> None:
        """Run scheduled indexing workflow."""
        while True:
            # Check for updates
            has_updates = await workflow.execute_activity(
                check_repository_updates,
                args=[repo_path],
                start_to_close_timeout=timedelta(minutes=5),
            )

            # Re-index if there are updates
            if has_updates:
                await workflow.execute_activity(
                    index_repository_activity,
                    args=[repo_path, None],
                    start_to_close_timeout=timedelta(hours=1),
                )

            # Wait for next interval
            await asyncio.sleep(interval_hours * 3600)


async def start_temporal_worker(config: Optional[CudgelConfig] = None) -> None:
    """Start a Temporal worker."""
    if config is None:
        config = get_config()

    # Connect to Temporal server
    client = await Client.connect(
        config.temporal_host,
        namespace=config.temporal_namespace,
    )

    # Create worker
    worker = Worker(
        client,
        task_queue=config.temporal_task_queue,
        workflows=[RepositoryIndexingWorkflow, ScheduledIndexingWorkflow],
        activities=[index_repository_activity, check_repository_updates],
    )

    # Run worker
    await worker.run()


async def schedule_repository_indexing(
    repo_path: str,
    name: Optional[str] = None,
    config: Optional[CudgelConfig] = None,
) -> str:
    """Schedule a repository for indexing."""
    if config is None:
        config = get_config()

    # Connect to Temporal
    client = await Client.connect(
        config.temporal_host,
        namespace=config.temporal_namespace,
    )

    # Start workflow
    handle = await client.start_workflow(
        RepositoryIndexingWorkflow.run,
        args=[repo_path, name],
        id=f"index-{repo_path}",
        task_queue=config.temporal_task_queue,
    )

    return handle.id


async def schedule_periodic_indexing(
    repo_path: str,
    interval_hours: int = 24,
    config: Optional[CudgelConfig] = None,
) -> str:
    """Schedule periodic indexing for a repository."""
    if config is None:
        config = get_config()

    # Connect to Temporal
    client = await Client.connect(
        config.temporal_host,
        namespace=config.temporal_namespace,
    )

    # Start workflow
    handle = await client.start_workflow(
        ScheduledIndexingWorkflow.run,
        args=[repo_path, interval_hours],
        id=f"scheduled-index-{repo_path}",
        task_queue=config.temporal_task_queue,
    )

    return handle.id

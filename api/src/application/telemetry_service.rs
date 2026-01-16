use crate::infrastructure::db::clickhouse::{ClickHouseRepository, ExecutionMetric, LogEntry};
use std::sync::Arc;

#[derive(Clone)]
pub struct TelemetryService {
    clickhouse_repository: Arc<ClickHouseRepository>,
}

impl TelemetryService {
    pub fn new(clickhouse_repository: Arc<ClickHouseRepository>) -> Self {
        Self {
            clickhouse_repository,
        }
    }

    pub async fn get_function_metrics(
        &self,
        function_name: &str,
    ) -> Result<Vec<ExecutionMetric>, String> {
        self.clickhouse_repository
            .get_function_executions(function_name, "1h")
            .await
            .map_err(|e| e.to_string())
            .map_err(|e| e.to_string())
    }

    pub async fn get_overall_metrics(&self) -> Result<Vec<ExecutionMetric>, String> {
        self.clickhouse_repository
            .get_overall_executions("1h")
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_function_logs(&self, function_name: &str) -> Result<Vec<LogEntry>, String> {
        self.clickhouse_repository
            .get_function_logs(function_name)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_recent_logs(&self) -> Result<Vec<LogEntry>, String> {
        self.clickhouse_repository
            .get_recent_logs()
            .await
            .map_err(|e| e.to_string())
    }
}

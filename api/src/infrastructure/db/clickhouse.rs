use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Row)]
pub struct ExecutionMetric {
    pub time_bucket: String,
    pub count: u64,
}

#[derive(Debug, Deserialize, Serialize, Row)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub body: String,
    pub trace_id: String,
    pub function_name: String,
}

#[derive(Clone)]
pub struct ClickHouseRepository {
    client: Client,
}

impl ClickHouseRepository {
    pub fn new(url: &str, user: &str, pass: &str, db: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_user(user)
            .with_password(pass)
            .with_database(db);
        Self { client }
    }

    pub async fn get_function_executions(
        &self,
        function_name: &str,
        _time_range: &str, // Unused for now
    ) -> anyhow::Result<Vec<ExecutionMetric>> {
        // Query to get execution counts per minute
        // We filter by the 'function_name' attribute we set in invocation_service.rs
        let query = "
            SELECT
                toString(toStartOfMinute(Timestamp)) as time_bucket,
                count() as count
            FROM otel_traces
            WHERE SpanAttributes['function_name'] = ?
            AND Timestamp > now() - toIntervalHour(1)
            GROUP BY time_bucket
            ORDER BY time_bucket
        ";

        let rows = self
            .client
            .query(query)
            .bind(function_name)
            .fetch_all::<ExecutionMetric>()
            .await?;

        Ok(rows)
    }

    pub async fn get_overall_executions(
        &self,
        _time_range: &str, // Unused for now
    ) -> anyhow::Result<Vec<ExecutionMetric>> {
        let query = "
            SELECT
                toString(toStartOfMinute(Timestamp)) as time_bucket,
                count() as count
            FROM otel_traces
            WHERE SpanAttributes['function_name'] != 'healthz'
            AND Timestamp > now() - toIntervalHour(1)
            GROUP BY time_bucket
            ORDER BY time_bucket
        ";

        let rows = self
            .client
            .query(query)
            .fetch_all::<ExecutionMetric>()
            .await?;

        Ok(rows)
    }

    pub async fn get_function_logs(&self, function_name: &str) -> anyhow::Result<Vec<LogEntry>> {
        let query = "
            SELECT
                toString(Timestamp) as timestamp,
                SeverityText as level,
                Body as body,
                TraceId as trace_id,
                LogAttributes['function_name'] as function_name
            FROM otel_logs
            WHERE LogAttributes['function_name'] = ?
            ORDER BY Timestamp DESC
            LIMIT 100
        ";

        let rows = self
            .client
            .query(query)
            .bind(function_name)
            .fetch_all::<LogEntry>()
            .await?;

        Ok(rows)
    }

    pub async fn get_recent_logs(&self) -> anyhow::Result<Vec<LogEntry>> {
        let query = "
            SELECT
                toString(Timestamp) as timestamp,
                SeverityText as level,
                Body as body,
                TraceId as trace_id,
                LogAttributes['function_name'] as function_name
            FROM otel_logs
            WHERE NOT (LogAttributes['function_name'] = 'healthz' AND SeverityText = 'INFO')
            ORDER BY Timestamp DESC
            LIMIT 50
        ";

        let rows = self.client.query(query).fetch_all::<LogEntry>().await?;

        Ok(rows)
    }
}

#![no_std]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;

// OpenTelemetry-compatible structures
#[derive(Debug, Clone)]
pub struct Span {
    pub trace_id: u128,
    pub span_id: u64,
    pub parent_span_id: Option<u64>,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub status: SpanStatus,
    pub attributes: BTreeMap<String, AttributeValue>,
    pub events: Vec<SpanEvent>,
}

#[derive(Debug, Clone)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

#[derive(Debug, Clone)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub timestamp: u64,
    pub name: String,
    pub attributes: BTreeMap<String, AttributeValue>,
}

#[derive(Debug, Clone)]
pub struct Trace {
    pub trace_id: u128,
    pub spans: Vec<Span>,
    pub root_span_id: u64,
}

pub struct TelemetryCollector {
    enabled: AtomicBool,
    traces: Mutex<BTreeMap<u128, Trace>>,
    active_spans: Mutex<BTreeMap<u64, Span>>,
    span_counter: AtomicU64,
    trace_counter: AtomicU64,
    sampling_rate: AtomicU64, // Percentage (0-100)
    exporters: Mutex<Vec<TelemetryExporter>>,
}

pub trait TelemetryExporter: Send + Sync {
    fn export_trace(&self, trace: &Trace);
    fn export_metric(&self, metric: &MetricData);
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct MetricData {
    pub name: String,
    pub description: String,
    pub unit: String,
    pub data_points: Vec<DataPoint>,
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub timestamp: u64,
    pub value: MetricValue,
    pub attributes: BTreeMap<String, AttributeValue>,
}

#[derive(Debug, Clone)]
pub enum MetricValue {
    Int(i64),
    Float(f64),
    Histogram(Vec<f64>),
}

// Prometheus exporter
pub struct PrometheusExporter {
    endpoint: String,
}

impl TelemetryExporter for PrometheusExporter {
    fn export_trace(&self, _trace: &Trace) {
        // Prometheus doesn't directly support traces
    }
    
    fn export_metric(&self, metric: &MetricData) {
        // Format and send metric in Prometheus format
        let _prometheus_format = format_prometheus_metric(metric);
        // Send to endpoint
    }
    
    fn name(&self) -> &str {
        "prometheus"
    }
}

fn format_prometheus_metric(metric: &MetricData) -> String {
    let mut output = String::new();
    output.push_str(&format!("# HELP {} {}\n", metric.name, metric.description));
    output.push_str(&format!("# TYPE {} gauge\n", metric.name));
    
    for point in &metric.data_points {
        let value = match &point.value {
            MetricValue::Int(v) => *v as f64,
            MetricValue::Float(v) => *v,
            MetricValue::Histogram(values) => {
                // Calculate average for simplicity
                values.iter().sum::<f64>() / values.len() as f64
            }
        };
        
        output.push_str(&format!("{} {}\n", metric.name, value));
    }
    
    output
}

static TELEMETRY_COLLECTOR: TelemetryCollector = TelemetryCollector {
    enabled: AtomicBool::new(false),
    traces: Mutex::new(BTreeMap::new()),
    active_spans: Mutex::new(BTreeMap::new()),
    span_counter: AtomicU64::new(0),
    trace_counter: AtomicU64::new(0),
    sampling_rate: AtomicU64::new(100),
    exporters: Mutex::new(Vec::new()),
};

pub fn init() {
    TELEMETRY_COLLECTOR.enabled.store(true, Ordering::SeqCst);
    set_sampling_rate(100); // Default to 100% sampling
}

pub fn shutdown() {
    TELEMETRY_COLLECTOR.enabled.store(false, Ordering::SeqCst);
    
    // Export any remaining traces
    export_all_traces();
}

pub fn set_sampling_rate(rate: u64) {
    let rate = rate.min(100);
    TELEMETRY_COLLECTOR.sampling_rate.store(rate, Ordering::SeqCst);
}

pub fn should_sample() -> bool {
    let rate = TELEMETRY_COLLECTOR.sampling_rate.load(Ordering::Relaxed);
    if rate >= 100 {
        return true;
    }
    
    // Simple sampling decision
    let random = crate::timer::get_ticks() % 100;
    random < rate
}

pub fn start_span(operation_name: &str, parent_span_id: Option<u64>) -> u64 {
    if !TELEMETRY_COLLECTOR.enabled.load(Ordering::Relaxed) {
        return 0;
    }
    
    if !should_sample() {
        return 0;
    }
    
    let span_id = TELEMETRY_COLLECTOR.span_counter.fetch_add(1, Ordering::SeqCst);
    let trace_id = if let Some(parent_id) = parent_span_id {
        // Get trace ID from parent
        if let Some(parent) = TELEMETRY_COLLECTOR.active_spans.lock().get(&parent_id) {
            parent.trace_id
        } else {
            generate_trace_id()
        }
    } else {
        generate_trace_id()
    };
    
    let span = Span {
        trace_id,
        span_id,
        parent_span_id,
        operation_name: operation_name.to_string(),
        start_time: crate::timer::get_ticks(),
        end_time: None,
        status: SpanStatus::Unset,
        attributes: BTreeMap::new(),
        events: Vec::new(),
    };
    
    TELEMETRY_COLLECTOR.active_spans.lock().insert(span_id, span);
    
    span_id
}

pub fn end_span(span_id: u64) {
    if !TELEMETRY_COLLECTOR.enabled.load(Ordering::Relaxed) {
        return;
    }
    
    let mut active_spans = TELEMETRY_COLLECTOR.active_spans.lock();
    if let Some(mut span) = active_spans.remove(&span_id) {
        span.end_time = Some(crate::timer::get_ticks());
        
        // Add to trace
        let mut traces = TELEMETRY_COLLECTOR.traces.lock();
        let trace = traces.entry(span.trace_id).or_insert_with(|| Trace {
            trace_id: span.trace_id,
            spans: Vec::new(),
            root_span_id: span_id,
        });
        
        trace.spans.push(span);
        
        // Export if trace is complete (no more active spans for this trace)
        let trace_complete = !active_spans
            .values()
            .any(|s| s.trace_id == trace.trace_id);
        
        if trace_complete {
            export_trace(&trace.clone());
            traces.remove(&trace.trace_id);
        }
    }
}

pub fn set_span_status(span_id: u64, status: SpanStatus) {
    if let Some(mut spans) = TELEMETRY_COLLECTOR.active_spans.try_lock() {
        if let Some(span) = spans.get_mut(&span_id) {
            span.status = status;
        }
    }
}

pub fn add_span_attribute(span_id: u64, key: &str, value: AttributeValue) {
    if let Some(mut spans) = TELEMETRY_COLLECTOR.active_spans.try_lock() {
        if let Some(span) = spans.get_mut(&span_id) {
            span.attributes.insert(key.to_string(), value);
        }
    }
}

pub fn add_span_event(span_id: u64, name: &str, attributes: BTreeMap<String, AttributeValue>) {
    if let Some(mut spans) = TELEMETRY_COLLECTOR.active_spans.try_lock() {
        if let Some(span) = spans.get_mut(&span_id) {
            span.events.push(SpanEvent {
                timestamp: crate::timer::get_ticks(),
                name: name.to_string(),
                attributes,
            });
        }
    }
}

fn generate_trace_id() -> u128 {
    let high = TELEMETRY_COLLECTOR.trace_counter.fetch_add(1, Ordering::SeqCst) as u128;
    let low = crate::timer::get_ticks() as u128;
    (high << 64) | low
}

pub fn record_metric(
    name: &str,
    value: MetricValue,
    attributes: BTreeMap<String, AttributeValue>,
) {
    if !TELEMETRY_COLLECTOR.enabled.load(Ordering::Relaxed) {
        return;
    }
    
    let mut data_points = Vec::new();
    data_points.push(DataPoint {
        timestamp: crate::timer::get_ticks(),
        value,
        attributes,
    });
    
    let metric = MetricData {
        name: name.to_string(),
        description: String::new(),
        unit: String::new(),
        data_points,
    };
    
    export_metric(&metric);
}

fn export_trace(trace: &Trace) {
    let exporters = TELEMETRY_COLLECTOR.exporters.lock();
    for exporter in exporters.iter() {
        exporter.export_trace(trace);
    }
}

fn export_metric(metric: &MetricData) {
    let exporters = TELEMETRY_COLLECTOR.exporters.lock();
    for exporter in exporters.iter() {
        exporter.export_metric(metric);
    }
}

fn export_all_traces() {
    let traces = TELEMETRY_COLLECTOR.traces.lock();
    for (_, trace) in traces.iter() {
        export_trace(trace);
    }
}

// Helper macros for instrumentation
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {{
        $crate::monitoring::telemetry::start_span($name, None)
    }};
    ($name:expr, $parent:expr) => {{
        $crate::monitoring::telemetry::start_span($name, Some($parent))
    }};
}

#[macro_export]
macro_rules! span_ok {
    ($span_id:expr) => {{
        $crate::monitoring::telemetry::set_span_status(
            $span_id,
            $crate::monitoring::telemetry::SpanStatus::Ok,
        );
        $crate::monitoring::telemetry::end_span($span_id);
    }};
}

#[macro_export]
macro_rules! span_error {
    ($span_id:expr) => {{
        $crate::monitoring::telemetry::set_span_status(
            $span_id,
            $crate::monitoring::telemetry::SpanStatus::Error,
        );
        $crate::monitoring::telemetry::end_span($span_id);
    }};
}

#[macro_export]
macro_rules! span_attr {
    ($span_id:expr, $key:expr, $value:expr) => {{
        $crate::monitoring::telemetry::add_span_attribute(
            $span_id,
            $key,
            $crate::monitoring::telemetry::AttributeValue::String($value.to_string()),
        );
    }};
}

pub fn get_active_spans() -> Vec<Span> {
    TELEMETRY_COLLECTOR
        .active_spans
        .lock()
        .values()
        .cloned()
        .collect()
}

pub fn get_trace(trace_id: u128) -> Option<Trace> {
    TELEMETRY_COLLECTOR.traces.lock().get(&trace_id).cloned()
}

pub fn clear_traces() {
    TELEMETRY_COLLECTOR.traces.lock().clear();
    TELEMETRY_COLLECTOR.active_spans.lock().clear();
}
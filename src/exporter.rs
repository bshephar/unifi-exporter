use crate::unifi::DeviceStats;
use anyhow::Result;
use prometheus::{Encoder, GaugeVec, Registry, TextEncoder};

pub struct MetricsExporter {
    pub registry: Registry,
    pub cpu_util: GaugeVec,
    pub mem_util: GaugeVec,
    pub uptime: GaugeVec,
    pub load_1: GaugeVec,
    pub load_5: GaugeVec,
    pub load_15: GaugeVec,
    pub tx_rate: GaugeVec,
    pub rx_rate: GaugeVec,
}

impl MetricsExporter {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        let cpu_util = GaugeVec::new(
            prometheus::Opts::new("unifi_device_cpu_utilization_pct", "CPU usage (%)"),
            &["device"],
        )?;
        let mem_util = GaugeVec::new(
            prometheus::Opts::new("unifi_device_memory_utilization_pct", "Memory usage (%)"),
            &["device"],
        )?;
        let uptime = GaugeVec::new(
            prometheus::Opts::new("unifi_device_uptime_seconds", "Uptime in seconds"),
            &["device"],
        )?;
        let load_1 = GaugeVec::new(
            prometheus::Opts::new("unifi_device_load_average_1min", "Load avg over 1min"),
            &["device"],
        )?;
        let load_5 = GaugeVec::new(
            prometheus::Opts::new("unifi_device_load_average_5min", "Load avg over 5min"),
            &["device"],
        )?;
        let load_15 = GaugeVec::new(
            prometheus::Opts::new("unifi_device_load_average_15min", "Load avg over 15min"),
            &["device"],
        )?;
        let tx_rate = GaugeVec::new(
            prometheus::Opts::new("unifi_device_tx_rate_bps", "TX rate in bps"),
            &["device"],
        )?;
        let rx_rate = GaugeVec::new(
            prometheus::Opts::new("unifi_device_rx_rate_bps", "RX rate in bps"),
            &["device"],
        )?;

        for metric in [
            &cpu_util, &mem_util, &uptime, &load_1, &load_5, &load_15, &tx_rate, &rx_rate,
        ] {
            registry.register(Box::new(metric.clone()))?;
        }

        Ok(Self {
            registry,
            cpu_util,
            mem_util,
            uptime,
            load_1,
            load_5,
            load_15,
            tx_rate,
            rx_rate,
        })
    }

    pub fn update_device_metrics(&self, device_name: &str, stats: &DeviceStats) {
        self.cpu_util
            .with_label_values(&[device_name])
            .set(stats.cpu_utilization_pct);
        self.mem_util
            .with_label_values(&[device_name])
            .set(stats.memory_utilization_pct);
        self.uptime
            .with_label_values(&[device_name])
            .set(stats.uptime_sec as f64);
        self.load_1
            .with_label_values(&[device_name])
            .set(stats.load_average_1min);
        self.load_5
            .with_label_values(&[device_name])
            .set(stats.load_average_5min);
        self.load_15
            .with_label_values(&[device_name])
            .set(stats.load_average_15min);
        self.tx_rate
            .with_label_values(&[device_name])
            .set(stats.uplink.tx_rate_bps as f64);
        self.rx_rate
            .with_label_values(&[device_name])
            .set(stats.uplink.rx_rate_bps as f64);
    }

    pub fn render(&self) -> Result<String> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        encoder.encode(&self.registry.gather(), &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

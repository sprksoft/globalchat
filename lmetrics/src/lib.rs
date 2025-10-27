#[cfg(feature = "rocket")]
mod httpmetrics;
#[cfg(feature = "nanohttp")]
mod nanohttp;
#[cfg(feature = "rocket")]
mod rocket;

pub use once_cell;
use prometheus::{
    core::Collector, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};

#[cfg(feature = "rocket")]
pub use {httpmetrics::*, rocket::*};

#[macro_export()]
macro_rules! register {
    ($($metric:path),+) => {
        {
            {
            let metrics = lmetrics::Metrics::new();
            $(metrics.register($metric.clone().into_collector());)+
            metrics
            }
        }
    };
}

#[macro_export()]
macro_rules! metrics_gen_extra_impl_items {
    (counter [$($label:ident),*]) => {};
    (gauge [$($label:ident),*]) => {
        pub fn set(value: i64, $($label: &str),*) {
            METRIC.set(value, &[$($label,)*]);
        }
    };
}

#[macro_export()]
macro_rules! metrics {


    ($vis:vis $type:ident $name:ident ($help:literal $(, [$($label:ident),*])?);) => {
        #[allow(dead_code)]
        #[allow(unused)]
        $vis mod $name {
            pub static METRIC: $crate::once_cell::sync::Lazy<$crate::Metric> = $crate::once_cell::sync::Lazy::new(|| {
                $crate::Metric::$type(stringify!($name), $help, &[$($(stringify!($label)),*)?])
            });
            pub fn inc($($($label: &str,)*)?){
                METRIC.inc(1, &[$($($label,)*)?]);
            }

            pub fn inc_by(count: u64, $($($label: &str,)*)?){
                METRIC.inc(count, &[$($($label,)*)?]);
            }
            $crate::metrics_gen_extra_impl_items!{$type [$($($label),*)?]}

        }
    };

    ($($vis:vis $type:ident $name:ident $args:tt;)*) => {
        $(
            $crate::metrics!($vis $type $name $args;);
        )*
    };

}

#[derive(Clone)]
pub enum Metric {
    Counter(IntCounter),
    CounterVec(IntCounterVec),
    Gauge(IntGauge),
    GaugeVec(IntGaugeVec),
}

impl Metric {
    pub fn counter(name: &str, help: &str, labels: &[&str]) -> Self {
        if labels.len() == 0 {
            Self::Counter(IntCounter::new(name, help).expect("Could not create metrics counter"))
        } else {
            Self::CounterVec(
                IntCounterVec::new(Opts::new(name, help), labels)
                    .expect("Could not create metrics counter vec"),
            )
        }
    }
    pub fn gauge(name: &str, help: &str, labels: &[&str]) -> Self {
        if labels.len() == 0 {
            Self::Gauge(IntGauge::new(name, help).expect("Could not create metrics gauge"))
        } else {
            Self::GaugeVec(
                IntGaugeVec::new(Opts::new(name, help), labels)
                    .expect("Could not create metrics gauge vec"),
            )
        }
    }

    pub fn set(&self, value: i64, labels: &[&str]) {
        match self {
            Self::Gauge(g) => g.set(value),
            Self::GaugeVec(g) => g.with_label_values(labels).set(value),
            _ => {
                panic!("Set is only supported on gauge metrics")
            }
        }
    }

    pub fn inc(&self, count: u64, labels: &[&str]) {
        match self {
            Self::Gauge(g) => g.add(count as i64),
            Self::GaugeVec(g) => {
                g.with_label_values(labels).add(count as i64);
            }
            Self::Counter(c) => {
                c.inc_by(count);
            }
            Self::CounterVec(c) => {
                c.with_label_values(labels).inc_by(count);
            }
        }
    }

    pub fn into_collector(self) -> Box<dyn Collector> {
        match self {
            Self::Gauge(g) => Box::new(g),
            Self::GaugeVec(g) => Box::new(g),
            Self::Counter(c) => Box::new(c),
            Self::CounterVec(c) => Box::new(c),
        }
    }
}

#[derive(Clone)]
pub struct LMetrics {
    pub registry: Registry,
}
impl LMetrics {
    pub fn new(metrics: &[&Metric]) -> Self {
        let me = Self::default();
        for met in metrics {
            me.register_metric(met);
        }
        me
    }
    pub fn register(&self, c: Box<dyn Collector>) {
        self.registry.register(c).unwrap();
    }
    pub fn register_metric(&self, metric: &Metric) {
        self.register(metric.clone().into_collector());
    }

    pub fn encode_metrics(&self) -> prometheus::Result<String> {
        let text_encoder = TextEncoder::new();
        let encoded = text_encoder.encode_to_string(&self.registry.gather())?;
        Ok(encoded)
    }
}
impl Default for LMetrics {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
        }
    }
}

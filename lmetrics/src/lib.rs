#[cfg(feature = "rocket")]
mod httpmetrics;
#[cfg(feature = "nanohttp")]
mod nanohttp;
#[cfg(feature = "rocket")]
mod rocket;

pub use once_cell;
use prometheus::{core::Collector, IntCounter, IntCounterVec, Opts, Registry, TextEncoder};

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
macro_rules! metrics {


    ($vis:vis counter $name:ident ($help:literal);) => {
        #[allow(dead_code)]
        #[allow(unused)]
        $vis mod $name {
            pub static METRIC: $crate::once_cell::sync::Lazy<$crate::Metric> = $crate::once_cell::sync::Lazy::new(|| {
                $crate::Metric::new_counter(stringify!($name), $help)
            });
            pub fn inc(){
                METRIC.inc(&[]);
            }
        }
    };

    ($vis:vis counter $name:ident ($help:literal, [$($label:ident),*]);) => {
        #[allow(dead_code)]
        #[allow(unused)]
        $vis mod $name {
            pub static METRIC: $crate::once_cell::sync::Lazy<$crate::Metric> = $crate::once_cell::sync::Lazy::new(|| {
                $crate::Metric::new_counter_vec(stringify!($name), $help, &[$(stringify!($label)),*])
            });
            pub fn inc($($label: &str,)*){
                METRIC.inc(&[$($label,)*]);
            }
        }
    };

    ($($vis:vis $type:ident $name:ident $args:tt;)*) => {
        $(
            metrics!($vis $type $name $args;);
        )*
    };

}

#[derive(Clone)]
pub enum Metric {
    Counter(IntCounter),
    CounterVec(IntCounterVec),
}

impl Metric {
    pub fn new_counter_vec(name: &str, help: &str, labels: &[&str]) -> Self {
        Self::CounterVec(
            IntCounterVec::new(Opts::new(name, help), labels).expect("Could not create counter"),
        )
    }
    pub fn new_counter(name: &str, help: &str) -> Self {
        Self::Counter(IntCounter::new(name, help).expect("Could not create counter"))
    }
    pub fn inc(&self, labels: &[&str]) {
        match self {
            Self::Counter(c) => {
                c.inc();
            }
            Self::CounterVec(c) => {
                c.with_label_values(labels).inc();
            }
        }
    }

    pub fn into_collector(self) -> Box<dyn Collector> {
        match self {
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

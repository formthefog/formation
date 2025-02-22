use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpuMetrics {
    pub index: usize,
    pub model: String,
    pub utilization_bps: u32, // GPU utilization in basis points (0-10000)
    pub memory_usage_bps: u32, // GPU memory usage in basis points (0-10000)
    pub temperature_deci_c: u32, // Temperature in 0.1°C units
    pub power_draw_deci_w: u32, // Power draw in 0.1W units
}

pub async fn collect_gpu_metrics() -> Result<Vec<GpuMetrics>, Box<dyn std::error::Error>> {
    let nvml = Nvml::init()?;
    let device_count = nvml.device_count()?;
    let mut gpus = Vec::new();

    for i in 0..device_count {
        let device = nvml.device_by_index(i)?;
        let index = i as usize;
        let model = device.name()?;
        let utilization = device.utilization_rates()?.gpu;
        let memory_info = device.memory_info()?;
        let memory_usage = ((memory_info.used as f64 / memory_info.total as f64) * 10000.0) as u32;
        let temperature = device.temperature(TemperatureSensor::Gpu)? * 10; // Deci-degrees
        let power_draw = (device.power_usage()? as f32 / 100.0) as u32;     // Deci-watts

        let gpu = GpuMetrics {
            index,
            model,
            utilization_bps: utilization * 100, // Convert to basis points
            memory_usage_bps: memory_usage,
            temperature_deci_c: temperature,
            power_draw_deci_w: power_draw,
        };
        gpus.push(gpu);
    }
    Ok(gpus)
}

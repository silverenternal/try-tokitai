use super::util::{live_source, now_ms, number};
use crate::process_window::CommandWindowExt;
use crate::visualization::model::{
    VisualizationDiagnostic, VisualizationDocument, VisualizationNode, VisualizationPoint,
    VisualizationSeries, VisualizationSource, VisualizationTypeDescriptor,
};
use crate::visualization::{type_descriptor, VisualizationAdapter, VisualizationContext};
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeMap;
use std::process::Command;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Once, RwLock};
use std::thread;
use std::time::Duration;
use sysinfo::{Networks, System};

pub struct SystemAdapter;

static SYSTEM_SAMPLER_START: Once = Once::new();
static SYSTEM_SNAPSHOT: RwLock<Option<Value>> = RwLock::new(None);
static SYSTEM_SAMPLER_LAST_ACCESS_MS: AtomicI64 = AtomicI64::new(0);

impl Default for SystemAdapter {
    fn default() -> Self {
        Self
    }
}

impl VisualizationAdapter for SystemAdapter {
    fn descriptor(&self) -> VisualizationTypeDescriptor {
        type_descriptor(
            "system",
            "System Performance",
            "Live CPU, GPU, memory, disk, process, container, and network metrics.",
            "atlas.system.runtime",
        )
    }

    fn discover(&self, _context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>> {
        touch_system_sampler();
        ensure_system_sampler();
        Ok(vec![live_source(
            "runtime:system",
            "system",
            "Local runtime",
            "runtime-monitor",
        )])
    }

    fn parse(&self, _context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
        touch_system_sampler();
        ensure_system_sampler();
        let source = live_source(
            "runtime:system",
            "system",
            "Local runtime",
            "runtime-monitor",
        );
        let mut document = VisualizationDocument::empty("system", "System Performance", source);
        let snapshot = SYSTEM_SNAPSHOT
            .read()
            .ok()
            .and_then(|snapshot| snapshot.clone())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "timestamp_ms": now_ms(),
                    "diagnostics": ["Collecting the first runtime sample."]
                })
            });
        append_metric_nodes(&snapshot, &mut document);
        document.metadata.insert("snapshot".to_string(), snapshot);
        Ok(document)
    }
}

fn touch_system_sampler() {
    SYSTEM_SAMPLER_LAST_ACCESS_MS.store(now_ms(), Ordering::Relaxed);
}

fn system_sampler_is_active() -> bool {
    now_ms().saturating_sub(SYSTEM_SAMPLER_LAST_ACCESS_MS.load(Ordering::Relaxed)) < 15_000
}

fn ensure_system_sampler() {
    SYSTEM_SAMPLER_START.call_once(|| {
        let _ = thread::Builder::new()
            .name("atlas-system-full-sampler".to_string())
            .spawn(|| loop {
                if !system_sampler_is_active() {
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
                publish_full_snapshot(collect_snapshot());
                thread::sleep(Duration::from_secs(10));
            });
        let _ = thread::Builder::new()
            .name("atlas-system-fast-sampler".to_string())
            .spawn(run_fast_sampler);
        let _ = thread::Builder::new()
            .name("atlas-system-gpu-sampler".to_string())
            .spawn(run_gpu_sampler);
        let _ = thread::Builder::new()
            .name("atlas-system-io-sampler".to_string())
            .spawn(run_io_sampler);
    });
}

#[cfg(windows)]
fn run_io_sampler() {
    loop {
        if !system_sampler_is_active() {
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        if let Some(snapshot) = collect_io_snapshot() {
            publish_io_snapshot(snapshot);
        }
        thread::sleep(Duration::from_millis(350));
    }
}

#[cfg(not(windows))]
fn run_io_sampler() {
    loop {
        thread::sleep(Duration::from_secs(2));
    }
}

#[cfg(windows)]
fn collect_io_snapshot() -> Option<Value> {
    let script = r#"
$ErrorActionPreference='SilentlyContinue'
[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$timestamp=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$diskPerf=@(Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk | Where-Object Name -ne '_Total')
$disks=@($diskPerf | ForEach-Object {
  $perf=$_
  $index=if($perf.Name -match '^(\d+)'){[int]$Matches[1]}else{$null}
  $volumes=if($perf.Name -match '^\d+\s+(.+)$'){$Matches[1]}else{''}
  @{timestamp_ms=$timestamp;label=if($null -ne $index){"Disk $index"}else{$perf.Name};volumes=$volumes;active_percent=[math]::Min(100,[double]$perf.PercentDiskTime);response_time_ms=[double]$perf.AvgDisksecPerTransfer*1000;read_bytes_per_sec=[double]$perf.DiskReadBytesPersec;write_bytes_per_sec=[double]$perf.DiskWriteBytesPersec}
})
$netPerf=@(Get-CimInstance Win32_PerfFormattedData_Tcpip_NetworkInterface | Where-Object {$_.Name -and $_.Name -ne '_Total'})
$netAdapters=@(Get-NetAdapter | Where-Object Status -eq 'Up')
$network=@($netPerf | ForEach-Object {
  $perf=$_
  $adapter=$netAdapters | Where-Object {$_.InterfaceDescription -eq $perf.Name -or $_.Name -eq $perf.Name} | Select-Object -First 1
  if(-not $adapter){$adapter=$netAdapters | Where-Object {$perf.Name -like ('*'+($_.Name -replace '[^a-zA-Z0-9]','*')+'*')} | Select-Object -First 1}
  $alias=if($adapter){$adapter.Name}else{$perf.Name}
  @{timestamp_ms=$timestamp;label=$alias;name=if($adapter){$adapter.InterfaceDescription}else{$perf.Name};receive_bytes_per_sec=[double]$perf.BytesReceivedPersec;send_bytes_per_sec=[double]$perf.BytesSentPersec}
})
@{disks=$disks;network_adapters=$network} | ConvertTo-Json -Depth 5 -Compress
"#;
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .hide_window()
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| {
            serde_json::from_str::<Value>(output.trim_start_matches('\u{feff}')).ok()
        })
}

fn publish_io_snapshot(incoming: Value) {
    let Ok(mut guard) = SYSTEM_SNAPSHOT.write() else {
        return;
    };
    let snapshot = guard.get_or_insert_with(|| {
        serde_json::json!({
            "timestamp_ms": now_ms(),
            "cpu": {},
            "memory": {},
            "disks": [],
            "network_adapters": []
        })
    });
    merge_runtime_array(snapshot, &incoming, "disks", &["label", "volumes"]);
    merge_runtime_array(snapshot, &incoming, "network_adapters", &["label", "name"]);
}

fn merge_runtime_array(target: &mut Value, incoming: &Value, key: &str, identity: &[&str]) {
    let Some(incoming) = incoming.get(key).and_then(Value::as_array) else {
        return;
    };
    if target.get(key).and_then(Value::as_array).is_none() {
        target[key] = Value::Array(Vec::new());
    }
    let Some(current) = target.get_mut(key).and_then(Value::as_array_mut) else {
        return;
    };
    for item in incoming {
        let existing = current.iter_mut().find(|candidate| {
            identity.iter().any(|field| {
                let value = item
                    .get(*field)
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty());
                value.is_some() && candidate.get(*field).and_then(Value::as_str) == value
            })
        });
        if let Some(existing) = existing {
            if let (Some(existing), Some(incoming)) = (existing.as_object_mut(), item.as_object()) {
                for (field, value) in incoming {
                    existing.insert(field.clone(), value.clone());
                }
            }
        } else {
            current.push(item.clone());
        }
    }
}

#[cfg(windows)]
fn run_gpu_sampler() {
    loop {
        if !system_sampler_is_active() {
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        if let Some(gpu) = collect_gpu_snapshot() {
            publish_gpu_snapshot(gpu);
        }
        thread::sleep(Duration::from_millis(350));
    }
}

#[cfg(not(windows))]
fn run_gpu_sampler() {
    loop {
        thread::sleep(Duration::from_secs(2));
    }
}

#[cfg(windows)]
fn collect_gpu_snapshot() -> Option<Value> {
    let script = r#"
$ErrorActionPreference='SilentlyContinue'
[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$timestamp=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$gpuEngines=@(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine | Where-Object {$_.Name -match '_phys_(\d+)_eng_\d+_engtype_(.+)$'})
$gpuMemory=@(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory)
$adapterIds=@($gpuEngines | ForEach-Object {if($_.Name -match '(luid_.+?_phys_\d+)_eng_'){$Matches[1]}} | Sort-Object -Unique)
$gpus=@($adapterIds | ForEach-Object {
  $adapterId=$_
  $engines=$gpuEngines | Where-Object {$_.Name -like "*${adapterId}_eng_*"}
  $engineValues=[ordered]@{}
  $engines | Group-Object {if($_.Name -match '_engtype_(.+)$'){$Matches[1]}else{'Engine'}} | ForEach-Object {$engineValues[$_.Name]=[double](($_.Group | Measure-Object UtilizationPercentage -Maximum).Maximum)}
  $util=[double](($engines | Measure-Object UtilizationPercentage -Maximum).Maximum)
  $memory=$gpuMemory | Where-Object Name -eq $adapterId | Select-Object -First 1
  @{adapter_id=$adapterId;timestamp_ms=$timestamp;label="GPU $([array]::IndexOf($adapterIds,$adapterId))";usage_percent=$util;engines=$engineValues;dedicated_usage_bytes=[double]$memory.DedicatedUsage;shared_usage_bytes=[double]$memory.SharedUsage;total_committed_bytes=[double]$memory.TotalCommitted}
})
@{adapters=$gpus} | ConvertTo-Json -Depth 6 -Compress
"#;
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .hide_window()
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| {
            serde_json::from_str::<Value>(output.trim_start_matches('\u{feff}')).ok()
        })
}

fn publish_gpu_snapshot(gpu: Value) {
    let Some(incoming) = gpu.get("adapters").and_then(Value::as_array) else {
        return;
    };
    let Ok(mut guard) = SYSTEM_SNAPSHOT.write() else {
        return;
    };
    let snapshot = guard.get_or_insert_with(|| {
        serde_json::json!({
            "timestamp_ms": now_ms(),
            "cpu": {},
            "memory": {},
            "network_adapters": [],
            "gpu": {"adapters": []}
        })
    });
    let Some(current) = snapshot
        .pointer_mut("/gpu/adapters")
        .and_then(Value::as_array_mut)
    else {
        snapshot["gpu"] = serde_json::json!({"adapters": incoming});
        return;
    };
    for adapter in incoming {
        let adapter_id = adapter.get("adapter_id").and_then(Value::as_str);
        let existing = current.iter_mut().find(|candidate| {
            adapter_id.is_some()
                && candidate.get("adapter_id").and_then(Value::as_str) == adapter_id
        });
        if let Some(existing) = existing {
            if let (Some(existing), Some(incoming)) =
                (existing.as_object_mut(), adapter.as_object())
            {
                for (key, value) in incoming {
                    existing.insert(key.clone(), value.clone());
                }
            }
        } else {
            current.push(adapter.clone());
        }
    }
}

fn publish_full_snapshot(snapshot: Value) {
    if let Ok(mut current) = SYSTEM_SNAPSHOT.write() {
        *current = Some(snapshot);
    }
}

fn run_fast_sampler() {
    let mut system = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();
    thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    loop {
        if !system_sampler_is_active() {
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        system.refresh_cpu_usage();
        system.refresh_memory();
        networks.refresh();
        publish_fast_snapshot(&system, &networks);
        thread::sleep(Duration::from_millis(750));
    }
}

fn publish_fast_snapshot(system: &System, networks: &Networks) {
    let cpu_usage = if system.cpus().is_empty() {
        None
    } else {
        Some(
            system
                .cpus()
                .iter()
                .map(|cpu| cpu.cpu_usage() as f64)
                .sum::<f64>()
                / system.cpus().len() as f64,
        )
    };
    let cpu_frequency = system.cpus().first().map(|cpu| cpu.frequency() as f64);
    let timestamp = now_ms();
    let Ok(mut guard) = SYSTEM_SNAPSHOT.write() else {
        return;
    };
    let snapshot = guard.get_or_insert_with(|| {
        serde_json::json!({
            "timestamp_ms": timestamp,
            "cpu": {},
            "memory": {},
            "network_adapters": []
        })
    });
    if let Some(value) = cpu_usage.filter(|value| value.is_finite()) {
        snapshot["cpu"]["usage_percent"] = Value::from(value);
    }
    if let Some(value) = cpu_frequency.filter(|value| value.is_finite() && *value > 0.0) {
        snapshot["cpu"]["speed_mhz"] = Value::from(value);
    }
    snapshot["cpu"]["logical_cores"] = Value::from(system.cpus().len() as u64);
    snapshot["cpu"]["timestamp_ms"] = Value::from(timestamp);
    snapshot["memory"]["total_bytes"] = Value::from(system.total_memory());
    snapshot["memory"]["available_bytes"] = Value::from(system.available_memory());
    snapshot["memory"]["timestamp_ms"] = Value::from(timestamp);
    merge_fast_networks(snapshot, networks);
}

fn merge_fast_networks(snapshot: &mut Value, networks: &Networks) {
    let elapsed_seconds = 0.75_f64;
    let Some(adapters) = snapshot
        .get_mut("network_adapters")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for (name, data) in networks {
        let Some(adapter) = adapters.iter_mut().find(|adapter| {
            adapter.get("label").and_then(Value::as_str) == Some(name.as_str())
                || adapter.get("name").and_then(Value::as_str) == Some(name.as_str())
        }) else {
            continue;
        };
        adapter["receive_bytes_per_sec"] = Value::from(data.received() as f64 / elapsed_seconds);
        adapter["send_bytes_per_sec"] = Value::from(data.transmitted() as f64 / elapsed_seconds);
        adapter["timestamp_ms"] = Value::from(now_ms());
    }
}

fn append_metric_nodes(snapshot: &Value, document: &mut VisualizationDocument) {
    let timestamp = snapshot
        .get("timestamp_ms")
        .and_then(Value::as_i64)
        .unwrap_or_else(now_ms);
    append_cpu(snapshot, document, timestamp);
    append_memory(snapshot, document, timestamp);
    append_disks(snapshot, document, timestamp);
    append_network(snapshot, document, timestamp);
    append_gpus(snapshot, document, timestamp);
    append_containers(snapshot, document, timestamp);
    if let Some(errors) = snapshot.get("diagnostics").and_then(Value::as_array) {
        for error in errors.iter().filter_map(Value::as_str) {
            document.diagnostics.push(VisualizationDiagnostic {
                level: "info".to_string(),
                message: error.to_string(),
                metadata: BTreeMap::new(),
            });
        }
    }
}

fn append_cpu(snapshot: &Value, document: &mut VisualizationDocument, timestamp: i64) {
    let Some(cpu) = snapshot.get("cpu") else {
        return;
    };
    let timestamp = sample_timestamp(cpu, timestamp);
    let Some(usage) = number(cpu.get("usage_percent")) else {
        return;
    };
    let details = vec![
        detail("Utilization", Some(usage), "%"),
        detail(
            "Speed",
            number(cpu.get("speed_mhz")).map(|v| v / 1000.0),
            "GHz",
        ),
        detail(
            "Processes",
            number(snapshot.pointer("/processes/count")),
            "",
        ),
        detail(
            "Threads",
            number(snapshot.pointer("/processes/thread_count")),
            "",
        ),
        detail(
            "Handles",
            number(snapshot.pointer("/processes/handle_count")),
            "",
        ),
        detail(
            "Up time",
            number(snapshot.pointer("/system/uptime_seconds")),
            "uptime",
        ),
        detail(
            "Base speed",
            number(cpu.get("base_speed_mhz")).map(|v| v / 1000.0),
            "GHz",
        ),
        detail("Sockets", number(cpu.get("sockets")), ""),
        detail("Cores", number(cpu.get("cores")), ""),
        detail("Logical processors", number(cpu.get("logical_cores")), ""),
        text_detail("Virtualization", cpu.get("virtualization")),
        detail("L1 cache", number(cpu.get("l1_cache_bytes")), "B"),
        detail("L2 cache", number(cpu.get("l2_cache_bytes")), "B"),
        detail("L3 cache", number(cpu.get("l3_cache_bytes")), "B"),
    ];
    append_resource(
        document,
        timestamp,
        "cpu",
        "CPU",
        "cpu",
        usage,
        "%",
        cpu.get("name").and_then(Value::as_str).unwrap_or_default(),
        "% Utilization",
        Some(100.0),
        details,
        vec![("cpu", "CPU utilization", "%", usage)],
    );
}

fn append_memory(snapshot: &Value, document: &mut VisualizationDocument, timestamp: i64) {
    let Some(memory) = snapshot.get("memory") else {
        return;
    };
    let timestamp = sample_timestamp(memory, timestamp);
    let Some(total) = number(memory.get("total_bytes")).filter(|v| *v > 0.0) else {
        return;
    };
    let Some(available) = number(memory.get("available_bytes")) else {
        return;
    };
    let used = (total - available).max(0.0);
    let details = vec![
        detail("In use", Some(used), "B"),
        detail("Available", Some(available), "B"),
        paired_detail(
            "Committed",
            number(memory.get("committed_bytes")),
            number(memory.get("commit_limit_bytes")),
            "B",
        ),
        detail("Cached", number(memory.get("cached_bytes")), "B"),
        detail("Paged pool", number(memory.get("paged_pool_bytes")), "B"),
        detail(
            "Non-paged pool",
            number(memory.get("nonpaged_pool_bytes")),
            "B",
        ),
        detail("Speed", number(memory.get("speed_mhz")), "MHz"),
        detail("Slots used", number(memory.get("slots_used")), ""),
        detail("Slots", number(memory.get("slots_total")), ""),
        text_detail("Form factor", memory.get("form_factor")),
        detail(
            "Hardware reserved",
            number(memory.get("hardware_reserved_bytes")),
            "B",
        ),
    ];
    append_resource(
        document,
        timestamp,
        "memory",
        "Memory",
        "memory",
        used,
        "B",
        "",
        "Memory usage",
        Some(total),
        details,
        vec![("memory", "Memory in use", "B", used)],
    );
}

fn append_disks(snapshot: &Value, document: &mut VisualizationDocument, timestamp: i64) {
    let Some(disks) = snapshot.get("disks").and_then(Value::as_array) else {
        return;
    };
    for (index, disk) in disks.iter().enumerate() {
        let resource_timestamp = sample_timestamp(disk, timestamp);
        let Some(active) = number(disk.get("active_percent")) else {
            continue;
        };
        let id = format!("disk:{index}");
        let mut label = disk
            .get("label")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("Disk {index}"));
        if let Some(volumes) = disk
            .get("volumes")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            label.push_str(&format!(" ({})", volumes.trim()));
        }
        let mut subtitle = disk
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if subtitle.is_empty() {
            subtitle = disk
                .get("media_type")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
        }
        let details = vec![
            detail("Active time", Some(active), "%"),
            detail(
                "Average response time",
                number(disk.get("response_time_ms")),
                "ms",
            ),
            detail("Read speed", number(disk.get("read_bytes_per_sec")), "B/s"),
            detail(
                "Write speed",
                number(disk.get("write_bytes_per_sec")),
                "B/s",
            ),
            detail("Capacity", number(disk.get("capacity_bytes")), "B"),
            text_detail("Type", disk.get("media_type")),
            text_detail("System disk", disk.get("system_disk")),
            text_detail("Page file", disk.get("page_file")),
        ];
        append_resource(
            document,
            resource_timestamp,
            &id,
            &label,
            "disk",
            active,
            "%",
            &subtitle,
            "% Active time",
            Some(100.0),
            details,
            vec![(&id, "Active time", "%", active)],
        );
    }
}

fn append_network(snapshot: &Value, document: &mut VisualizationDocument, timestamp: i64) {
    let Some(adapters) = snapshot.get("network_adapters").and_then(Value::as_array) else {
        return;
    };
    for (index, adapter) in adapters.iter().enumerate() {
        let resource_timestamp = sample_timestamp(adapter, timestamp);
        let received = number(adapter.get("receive_bytes_per_sec"));
        let sent = number(adapter.get("send_bytes_per_sec"));
        let Some(primary) = received.or(sent) else {
            continue;
        };
        let id = format!("network:{index}");
        let label = adapter
            .get("label")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .unwrap_or("Ethernet");
        let subtitle = adapter
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let details = vec![
            detail("Send", sent, "B/s"),
            detail("Receive", received, "B/s"),
            detail(
                "Link speed",
                number(adapter.get("link_speed_bits_per_sec")),
                "b/s",
            ),
            text_detail("IPv4 address", adapter.get("ipv4")),
            text_detail("IPv6 address", adapter.get("ipv6")),
        ];
        let mut series = Vec::new();
        if let Some(value) = received {
            series.push((format!("{id}:receive"), "Receive", "B/s", value));
        }
        if let Some(value) = sent {
            series.push((format!("{id}:send"), "Send", "B/s", value));
        }
        append_resource_owned(
            document,
            resource_timestamp,
            &id,
            label,
            "network",
            primary,
            "B/s",
            subtitle,
            "Throughput",
            None,
            details,
            series,
        );
    }
}

fn append_gpus(snapshot: &Value, document: &mut VisualizationDocument, timestamp: i64) {
    let adapters = snapshot
        .pointer("/gpu/adapters")
        .and_then(Value::as_array)
        .or_else(|| snapshot.pointer("/gpu/devices").and_then(Value::as_array));
    let Some(adapters) = adapters else { return };
    for (index, gpu) in adapters.iter().enumerate() {
        let resource_timestamp = sample_timestamp(gpu, timestamp);
        let Some(usage) = number(gpu.get("usage_percent")) else {
            continue;
        };
        let id = format!("gpu:{index}");
        let label = gpu
            .get("label")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("GPU {index}"));
        let subtitle = gpu.get("name").and_then(Value::as_str).unwrap_or_default();
        let details = vec![
            detail("Utilization", Some(usage), "%"),
            detail(
                "Dedicated GPU memory",
                number(gpu.get("dedicated_usage_bytes"))
                    .or_else(|| number(gpu.get("memory_used_mb")).map(|v| v * 1024.0 * 1024.0)),
                "B",
            ),
            detail(
                "Dedicated GPU memory capacity",
                number(gpu.get("dedicated_total_bytes"))
                    .or_else(|| number(gpu.get("memory_total_mb")).map(|v| v * 1024.0 * 1024.0)),
                "B",
            ),
            detail(
                "Shared GPU memory",
                number(gpu.get("shared_usage_bytes")),
                "B",
            ),
            detail(
                "Total committed",
                number(gpu.get("total_committed_bytes")),
                "B",
            ),
            text_detail("Driver version", gpu.get("driver_version")),
            text_detail("Driver date", gpu.get("driver_date")),
            text_detail("Physical location", gpu.get("physical_location")),
        ];
        let mut series = Vec::new();
        if let Some(engines) = gpu.get("engines").and_then(Value::as_object) {
            for (name, value) in engines {
                if let Some(value) = number(Some(value)) {
                    series.push((
                        format!("{id}:{}", safe_fragment(name)),
                        name.as_str(),
                        "%",
                        value,
                    ));
                }
            }
        }
        if series.is_empty() {
            series.push((id.clone(), "GPU utilization", "%", usage));
        }
        append_resource_owned(
            document,
            resource_timestamp,
            &id,
            &label,
            "gpu",
            usage,
            "%",
            subtitle,
            "% Utilization",
            Some(100.0),
            details,
            series,
        );
    }
}

fn append_containers(snapshot: &Value, document: &mut VisualizationDocument, timestamp: i64) {
    let Some(count) = number(snapshot.pointer("/docker/running_containers")) else {
        return;
    };
    append_resource(
        document,
        timestamp,
        "docker",
        "Docker",
        "container",
        count,
        "",
        "Running containers",
        "Containers",
        None,
        vec![detail("Running", Some(count), "")],
        vec![("docker", "Running containers", "", count)],
    );
}

fn append_resource<'a>(
    document: &mut VisualizationDocument,
    timestamp: i64,
    id: &str,
    label: &str,
    category: &str,
    value: f64,
    unit: &str,
    subtitle: &str,
    graph_label: &str,
    graph_max: Option<f64>,
    details: Vec<Value>,
    series: Vec<(&'a str, &'a str, &'a str, f64)>,
) {
    append_resource_owned(
        document,
        timestamp,
        id,
        label,
        category,
        value,
        unit,
        subtitle,
        graph_label,
        graph_max,
        details,
        series
            .into_iter()
            .map(|(id, label, unit, value)| (id.to_string(), label, unit, value))
            .collect(),
    );
}

fn append_resource_owned<L: AsRef<str>, U: AsRef<str>>(
    document: &mut VisualizationDocument,
    timestamp: i64,
    id: &str,
    label: &str,
    category: &str,
    value: f64,
    unit: &str,
    subtitle: &str,
    graph_label: &str,
    graph_max: Option<f64>,
    details: Vec<Value>,
    series: Vec<(String, L, U, f64)>,
) {
    if !value.is_finite() {
        return;
    }
    let node_id = format!("metric:{id}");
    let mut node = VisualizationNode::new(&node_id, label, category);
    node.status = "live".to_string();
    node.metrics.insert("value".to_string(), value);
    node.metadata.insert(
        "presentation".to_string(),
        Value::String("task-manager-performance".to_string()),
    );
    node.metadata
        .insert("unit".to_string(), Value::String(unit.to_string()));
    node.metadata
        .insert("subtitle".to_string(), Value::String(subtitle.to_string()));
    node.metadata.insert(
        "graph_label".to_string(),
        Value::String(graph_label.to_string()),
    );
    node.metadata.insert(
        "details".to_string(),
        Value::Array(details.into_iter().filter(|item| !item.is_null()).collect()),
    );
    if let Some(max) = graph_max.filter(|v| v.is_finite() && *v > 0.0) {
        node.metadata
            .insert("graph_max".to_string(), Value::from(max));
    }
    document.nodes.push(node);
    for (series_id, series_label, series_unit, series_value) in series {
        if !series_value.is_finite() {
            continue;
        }
        document.series.push(VisualizationSeries {
            id: series_id,
            label: series_label.as_ref().to_string(),
            unit: series_unit.as_ref().to_string(),
            node_id: Some(node_id.clone()),
            category: category.to_string(),
            points: vec![VisualizationPoint {
                timestamp_ms: timestamp,
                value: series_value,
            }],
        });
    }
}

fn detail(label: &str, value: Option<f64>, unit: &str) -> Value {
    value
        .filter(|v| v.is_finite())
        .map(|value| serde_json::json!({"label": label, "value": value, "unit": unit}))
        .unwrap_or(Value::Null)
}

fn paired_detail(label: &str, value: Option<f64>, secondary: Option<f64>, unit: &str) -> Value {
    match (
        value.filter(|v| v.is_finite()),
        secondary.filter(|v| v.is_finite()),
    ) {
        (Some(value), Some(secondary_value)) => {
            serde_json::json!({"label": label, "value": value, "secondary_value": secondary_value, "unit": unit})
        }
        (Some(value), None) => serde_json::json!({"label": label, "value": value, "unit": unit}),
        _ => Value::Null,
    }
}

fn text_detail(label: &str, value: Option<&Value>) -> Value {
    let text = value.and_then(|value| {
        value.as_str().map(str::to_string).or_else(|| {
            value
                .as_bool()
                .map(|value| if value { "Yes" } else { "No" }.to_string())
        })
    });
    text.filter(|value| !value.trim().is_empty())
        .map(|value| serde_json::json!({"label": label, "value": value}))
        .unwrap_or(Value::Null)
}

fn safe_fragment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
}

fn sample_timestamp(value: &Value, fallback: i64) -> i64 {
    value
        .get("timestamp_ms")
        .and_then(Value::as_i64)
        .unwrap_or(fallback)
}

#[cfg(windows)]
fn collect_snapshot() -> Value {
    let script = r#"
$ErrorActionPreference='SilentlyContinue'
[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$sampleTimestamp=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$os=Get-CimInstance Win32_OperatingSystem
$all=Get-Process
$cpuInfo=Get-CimInstance Win32_Processor | Select-Object -First 1
$cpuPerf=Get-CimInstance Win32_PerfFormattedData_Counters_ProcessorInformation | Where-Object Name -eq '_Total' | Select-Object -First 1
if(-not $cpuPerf){$cpuPerf=Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor | Where-Object Name -eq '_Total' | Select-Object -First 1}
$caches=Get-CimInstance Win32_CacheMemory
$l1=($caches | Where-Object Level -eq 3 | Measure-Object MaxCacheSize -Sum).Sum*1024
$l2=($caches | Where-Object Level -eq 4 | Measure-Object MaxCacheSize -Sum).Sum*1024
$l3=($caches | Where-Object Level -eq 5 | Measure-Object MaxCacheSize -Sum).Sum*1024
$memPerf=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory | Select-Object -First 1
$modules=@(Get-CimInstance Win32_PhysicalMemory)
$installedMemory=[double](($modules | Measure-Object Capacity -Sum).Sum)
$arrays=Get-CimInstance Win32_PhysicalMemoryArray | Measure-Object MemoryDevices -Sum
$diskPerf=@(Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk | Where-Object Name -ne '_Total')
$diskInfo=@(Get-CimInstance Win32_DiskDrive | Sort-Object Index)
$disks=@($diskPerf | ForEach-Object {
  $perf=$_
  $index=if($perf.Name -match '^(\d+)'){[int]$Matches[1]}else{$null}
  $info=$diskInfo | Where-Object Index -eq $index | Select-Object -First 1
  $volumes=if($perf.Name -match '^\d+\s+(.+)$'){$Matches[1]}else{''}
  @{label=if($null -ne $index){"Disk $index"}else{$perf.Name};volumes=$volumes;model=$info.Model;media_type=$info.MediaType;capacity_bytes=[double]$info.Size;active_percent=[math]::Min(100,[double]$perf.PercentDiskTime);response_time_ms=[double]$perf.AvgDisksecPerTransfer*1000;read_bytes_per_sec=[double]$perf.DiskReadBytesPersec;write_bytes_per_sec=[double]$perf.DiskWriteBytesPersec;system_disk=($volumes -match '(^|\s)C:($|\s)');page_file=($volumes -match '(^|\s)C:($|\s)')}
})
$netPerf=@(Get-CimInstance Win32_PerfFormattedData_Tcpip_NetworkInterface | Where-Object {$_.Name -and $_.Name -ne '_Total'})
$netAdapters=@(Get-NetAdapter | Where-Object Status -eq 'Up')
$ipAddresses=@(Get-NetIPAddress | Where-Object AddressState -eq 'Preferred')
$network=@($netPerf | ForEach-Object {
  $perf=$_
  $adapter=$netAdapters | Where-Object {$_.InterfaceDescription -eq $perf.Name -or $_.Name -eq $perf.Name} | Select-Object -First 1
  if(-not $adapter){$adapter=$netAdapters | Where-Object {$perf.Name -like ('*'+($_.Name -replace '[^a-zA-Z0-9]','*')+'*')} | Select-Object -First 1}
  $alias=if($adapter){$adapter.Name}else{$perf.Name}
  $addresses=$ipAddresses | Where-Object InterfaceAlias -eq $alias
  @{label=$alias;name=if($adapter){$adapter.InterfaceDescription}else{$perf.Name};receive_bytes_per_sec=[double]$perf.BytesReceivedPersec;send_bytes_per_sec=[double]$perf.BytesSentPersec;link_speed_bits_per_sec=if($adapter){[double]$(if($adapter.LinkSpeed -match '([\d\.]+)\s*Gbps'){$Matches[1]*1000000000}elseif($adapter.LinkSpeed -match '([\d\.]+)\s*Mbps'){$Matches[1]*1000000}else{0})}else{$null};ipv4=($addresses | Where-Object AddressFamily -eq 'IPv4' | Select-Object -First 1 -ExpandProperty IPAddress);ipv6=($addresses | Where-Object AddressFamily -eq 'IPv6' | Select-Object -First 1 -ExpandProperty IPAddress)}
})
$video=@(Get-CimInstance Win32_VideoController | Where-Object Name)
$gpuEngines=@(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine | Where-Object {$_.Name -match '_phys_(\d+)_eng_\d+_engtype_(.+)$'})
$gpuMemory=@(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory)
$adapterIds=@($gpuEngines | ForEach-Object {if($_.Name -match '(luid_.+?_phys_\d+)_eng_'){$Matches[1]}} | Sort-Object -Unique)
$gpus=@($adapterIds | ForEach-Object {
  $adapterId=$_
  $engines=$gpuEngines | Where-Object {$_.Name -like "*${adapterId}_eng_*"}
  $engineValues=[ordered]@{}
  $engines | Group-Object {if($_.Name -match '_engtype_(.+)$'){$Matches[1]}else{'Engine'}} | ForEach-Object {$engineValues[$_.Name]=[double](($_.Group | Measure-Object UtilizationPercentage -Maximum).Maximum)}
  $util=[double](($engines | Measure-Object UtilizationPercentage -Maximum).Maximum)
  $memory=$gpuMemory | Where-Object Name -eq $adapterId | Select-Object -First 1
  $controller=if($video.Count -eq $adapterIds.Count){$video | Select-Object -Index ([array]::IndexOf($adapterIds,$adapterId))}else{$null}
  @{adapter_id=$adapterId;timestamp_ms=$sampleTimestamp;label="GPU $([array]::IndexOf($adapterIds,$adapterId))";name=$controller.Name;usage_percent=$util;engines=$engineValues;dedicated_usage_bytes=[double]$memory.DedicatedUsage;shared_usage_bytes=[double]$memory.SharedUsage;total_committed_bytes=[double]$memory.TotalCommitted;dedicated_total_bytes=[double]$controller.AdapterRAM;driver_version=$controller.DriverVersion;driver_date=$controller.DriverDate;physical_location=$controller.PNPDeviceID}
})
$handles=($all | Measure-Object HandleCount -Sum).Sum
$threads=($all | ForEach-Object {$_.Threads.Count} | Measure-Object -Sum).Sum
$totalMemory=[double]$os.TotalVisibleMemorySize*1024
$availableMemory=[double]$os.FreePhysicalMemory*1024
$obj=[ordered]@{
  timestamp_ms=$sampleTimestamp
  system=@{uptime_seconds=([DateTime]::Now-$os.LastBootUpTime).TotalSeconds}
  cpu=@{name=$cpuInfo.Name;usage_percent=[double]$(if($null -ne $cpuPerf.PercentProcessorUtility){$cpuPerf.PercentProcessorUtility}else{$cpuPerf.PercentProcessorTime});speed_mhz=[double]$(if($cpuPerf.ActualFrequency){$cpuPerf.ActualFrequency}else{$cpuInfo.CurrentClockSpeed});base_speed_mhz=[double]$cpuInfo.MaxClockSpeed;sockets=(Get-CimInstance Win32_ComputerSystem).NumberOfProcessors;cores=[double]$cpuInfo.NumberOfCores;logical_cores=[double]$cpuInfo.NumberOfLogicalProcessors;virtualization=if($cpuInfo.VirtualizationFirmwareEnabled){'Enabled'}else{'Disabled'};l1_cache_bytes=[double]$l1;l2_cache_bytes=[double]$l2;l3_cache_bytes=[double]$l3}
  memory=@{total_bytes=$totalMemory;available_bytes=$availableMemory;committed_bytes=[double]$memPerf.CommittedBytes;commit_limit_bytes=[double]$memPerf.CommitLimit;cached_bytes=[double]$memPerf.CacheBytes;paged_pool_bytes=[double]$memPerf.PoolPagedBytes;nonpaged_pool_bytes=[double]$memPerf.PoolNonpagedBytes;speed_mhz=[double](($modules | Measure-Object ConfiguredClockSpeed -Maximum).Maximum);slots_used=$modules.Count;slots_total=[double]$arrays.Sum;form_factor=($modules | Select-Object -First 1 -ExpandProperty FormFactor);hardware_reserved_bytes=[math]::Max(0,$installedMemory-$totalMemory)}
  disks=$disks
  network_adapters=$network
  gpu=@{adapters=$gpus}
  processes=@{count=$all.Count;thread_count=$threads;handle_count=$handles}
}
$obj | ConvertTo-Json -Depth 6 -Compress
"#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .hide_window()
        .output();
    let mut snapshot = output
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| serde_json::from_str::<Value>(output.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_else(|| serde_json::json!({"timestamp_ms": now_ms(), "diagnostics": ["Windows performance counters were unavailable."]}));
    append_gpu(&mut snapshot);
    append_docker(&mut snapshot);
    snapshot
}

#[cfg(not(windows))]
fn collect_snapshot() -> Value {
    let mut diagnostics = Vec::new();
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|raw| raw.split_whitespace().next()?.parse::<f64>().ok());
    let memory = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut memory_values = BTreeMap::new();
    for line in memory.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(key), Some(value)) = (
            parts.next(),
            parts.next().and_then(|v| v.parse::<f64>().ok()),
        ) {
            memory_values.insert(key.trim_end_matches(':').to_string(), value * 1024.0);
        }
    }
    let total = *memory_values.get("MemTotal").unwrap_or(&0.0);
    let available = *memory_values
        .get("MemAvailable")
        .or_else(|| memory_values.get("MemFree"))
        .unwrap_or(&0.0);
    let cores = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let disk = Command::new("df")
        .args(["-Pk", "/"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|raw| raw.lines().nth(1).map(str::to_string));
    let disk_parts = disk
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>();
    let disk_total = disk_parts
        .get(1)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0)
        * 1024.0;
    let disk_free = disk_parts
        .get(3)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0)
        * 1024.0;
    if total == 0.0 {
        diagnostics.push("Memory metrics were unavailable.".to_string());
    }
    let mut snapshot = serde_json::json!({
        "timestamp_ms": now_ms(),
        "cpu": {"usage_percent": load.map(|value| value / cores as f64 * 100.0), "logical_cores": cores, "load_1m": load},
        "memory": {"total_bytes": total, "available_bytes": available, "usage_percent": if total > 0.0 { (1.0 - available / total) * 100.0 } else { 0.0 }},
        "disk": {"total_bytes": disk_total, "free_bytes": disk_free, "usage_percent": if disk_total > 0.0 { (1.0 - disk_free / disk_total) * 100.0 } else { 0.0 }},
        "diagnostics": diagnostics
    });
    append_gpu(&mut snapshot);
    append_docker(&mut snapshot);
    snapshot
}

fn append_gpu(snapshot: &mut Value) {
    if snapshot
        .pointer("/gpu/adapters")
        .and_then(Value::as_array)
        .is_some_and(|adapters| !adapters.is_empty())
    {
        return;
    }
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,memory.used,memory.total,name",
            "--format=csv,noheader,nounits",
        ])
        .hide_window()
        .output();
    let Some(raw) = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
    else {
        return;
    };
    let rows = raw.lines().filter_map(|line| {
        let values = line.split(',').map(str::trim).collect::<Vec<_>>();
        Some(serde_json::json!({"usage_percent": values.first()?.parse::<f64>().ok()?, "memory_used_mb": values.get(1)?.parse::<f64>().ok()?, "memory_total_mb": values.get(2)?.parse::<f64>().ok()?, "name": values.get(3).copied().unwrap_or("GPU")}))
    }).collect::<Vec<_>>();
    if rows.is_empty() {
        return;
    }
    let usage = rows
        .iter()
        .filter_map(|value| number(value.get("usage_percent")))
        .sum::<f64>()
        / rows.len() as f64;
    let used = rows
        .iter()
        .filter_map(|value| number(value.get("memory_used_mb")))
        .sum::<f64>();
    let total = rows
        .iter()
        .filter_map(|value| number(value.get("memory_total_mb")))
        .sum::<f64>();
    snapshot["gpu"] = serde_json::json!({"usage_percent": usage, "memory_usage_percent": if total > 0.0 { used / total * 100.0 } else { 0.0 }, "devices": rows});
}

fn append_docker(snapshot: &mut Value) {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{json .}}"])
        .hide_window()
        .output();
    let Some(raw) = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
    else {
        return;
    };
    let containers = raw
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    snapshot["docker"] =
        serde_json::json!({"running_containers": containers.len(), "containers": containers});
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_runtime_metrics_do_not_create_empty_resources() {
        let source = live_source(
            "runtime:system",
            "system",
            "Local runtime",
            "runtime-monitor",
        );
        let mut document = VisualizationDocument::empty("system", "System Performance", source);
        append_metric_nodes(
            &serde_json::json!({
                "timestamp_ms": 1,
                "cpu": {"usage_percent": 12.5},
                "memory": {"available_bytes": 1024},
                "disks": [{"label": "Disk 0"}],
                "network_adapters": [{"label": "Ethernet"}],
                "gpu": {"adapters": [{"label": "GPU 0"}]}
            }),
            &mut document,
        );

        assert_eq!(document.nodes.len(), 1);
        assert_eq!(document.nodes[0].id, "metric:cpu");
        assert_eq!(document.series.len(), 1);
        assert!(document.series[0]
            .points
            .iter()
            .all(|point| point.value.is_finite()));
    }
}

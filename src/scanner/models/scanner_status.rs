use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScannerState {
    Idle,
    Processing,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdfState {
    Empty,
    Loaded,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Processing,
    Completed,
    Canceled,
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub job_uri: String,
    pub job_state: JobState,
    pub job_state_reason: String,
    pub images_completed: u32,
}

#[derive(Debug, Clone)]
pub struct ScannerStatus {
    pub state: ScannerState,
    pub adf_state: AdfState,
    pub jobs: Vec<JobInfo>,
}

/// Parse `/eSCL/ScannerStatus` XML.
pub fn parse_scanner_status(xml: &str) -> Result<ScannerStatus> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    let mut state = ScannerState::Unknown(String::new());
    let mut adf_state = AdfState::Unknown;
    let mut jobs = Vec::new();

    for node in root.children() {
        match node.tag_name().name() {
            "State" => {
                state = match node.text().unwrap_or("") {
                    "Idle" => ScannerState::Idle,
                    "Processing" => ScannerState::Processing,
                    other => ScannerState::Unknown(other.to_string()),
                };
            }
            "AdfState" => {
                adf_state = match node.text().unwrap_or("") {
                    "ScannerAdfEmpty" | "Empty" => AdfState::Empty,
                    "ScannerAdfLoaded" | "Loaded" => AdfState::Loaded,
                    _ => AdfState::Unknown,
                };
            }
            "Jobs" => {
                for job_node in node.descendants() {
                    if job_node.tag_name().name() == "JobInfo" {
                        jobs.push(parse_job_info(job_node));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ScannerStatus {
        state,
        adf_state,
        jobs,
    })
}

fn parse_job_info(node: roxmltree::Node) -> JobInfo {
    let mut job_uri = String::new();
    let mut job_state = JobState::Unknown(String::new());
    let mut job_state_reason = String::new();
    let mut images_completed = 0u32;

    for child in node.children() {
        match child.tag_name().name() {
            "JobUri" => job_uri = child.text().unwrap_or("").to_string(),
            "JobState" => {
                job_state = match child.text().unwrap_or("") {
                    "Processing" => JobState::Processing,
                    "Completed" => JobState::Completed,
                    "Canceled" => JobState::Canceled,
                    other => JobState::Unknown(other.to_string()),
                };
            }
            "JobStateReasons" | "JobStateReason" => {
                job_state_reason = child.text().unwrap_or("").to_string();
            }
            "ImagesCompleted" => {
                images_completed = child.text().and_then(|t| t.parse().ok()).unwrap_or(0);
            }
            _ => {}
        }
    }

    JobInfo {
        job_uri,
        job_state,
        job_state_reason,
        images_completed,
    }
}

/// Parse `/eSCL/ScanJobs/{id}/ScanImageInfo` XML to get actual page dimensions.
#[derive(Debug, Clone)]
pub struct ScanImageInfo {
    pub actual_width: u32,
    pub actual_height: u32,
}

pub fn parse_scan_image_info(xml: &str) -> Result<ScanImageInfo> {
    let doc = roxmltree::Document::parse(xml)?;
    let mut width = 0u32;
    let mut height = 0u32;

    for node in doc.root_element().descendants() {
        match node.tag_name().name() {
            "ActualWidth" => width = node.text().and_then(|t| t.parse().ok()).unwrap_or(0),
            "ActualHeight" => height = node.text().and_then(|t| t.parse().ok()).unwrap_or(0),
            _ => {}
        }
    }

    Ok(ScanImageInfo {
        actual_width: width,
        actual_height: height,
    })
}

use anyhow::Result;
use async_trait::async_trait;
use itertools::izip;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum AccessionCodes {
    Srx(Srx),
    Srr(Srr),
    Gsm(Gsm),
}

#[derive(Debug, Clone)]
pub struct Srx(String);
#[derive(Debug, Clone)]
pub struct Srr(String);
#[derive(Debug, Clone)]
pub struct Gsm(String);

impl AccessionCodes {
    pub fn from_str(s: &str) -> Result<Self> {
        let prefix = &s[..3];
        match prefix {
            "SRX" => Ok(AccessionCodes::Srx(Srx(s.to_string()))),
            "SRR" => Ok(AccessionCodes::Srr(Srr(s.to_string()))),
            "GSM" => Ok(AccessionCodes::Gsm(Gsm(s.to_string()))),
            _ => Err(anyhow::anyhow!("unknown prefix: {}", prefix)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EnaRecord {
    pub experiment_accession: String,
    pub run_accession: String,
    pub fastq_md5: String,
    pub fastq_ftp: String,
    pub fastq_aspera: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DownloadInfo {
    pub orig_acc: String,
    pub run_acc: String,
    pub http_url: String,
    pub md5: String,
    pub ascp_url: String,
}

impl DownloadInfo {
    pub fn to_aria2(&self) -> String {
        let mut aria2 = String::new();
        aria2.push_str(format!("https://{}", self.http_url).as_str());
        aria2.push_str("\n ");
        aria2.push_str(format!("checksum=md5={}", self.md5).as_str());
        aria2.push_str("\n ");
        aria2.push_str("check-integrity=true");
        aria2.push_str("\n ");
        aria2.push_str(format!("out={}.fastq.gz", self.orig_acc).as_str());
        aria2.push_str("\n");
        aria2
    }
    pub fn to_ascp(&self) -> String {
        // just output json with serde_json
        serde_json::to_string(self).unwrap()
    }
}

#[async_trait]
pub trait DownloadableAccession {
    fn orig_accession(&self) -> String;
    async fn get_downloadable_accession(&self) -> Option<String> {
        Some(self.orig_accession())
    }
    async fn get_download_info(&self) -> Option<Vec<DownloadInfo>> {
        let json_url = "https://www.ebi.ac.uk/ena/portal/api/filereport";
        if let Some(downloadable_accession) = self.get_downloadable_accession().await {
            let params = [
                ("accession", downloadable_accession.as_str()),
                ("result", "read_run"),
                ("format", "json"),
                (
                    "fields",
                    "experiment_accession,run_accession,fastq_ftp,fastq_md5,fastq_aspera",
                ),
            ];
            let client = reqwest::Client::new();
            let resp: Vec<EnaRecord> = client
                .get(json_url)
                .query(&params)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            // md5, aspera link, ftp link are separated by ";", so we need to split them and then concat resulted vec
            // into a single vec
            let mut download_info_list = Vec::new();
            for record in resp {
                let md5_vec: Vec<String> =
                    record.fastq_md5.split(";").map(|x| x.to_string()).collect();
                let http_url_vec: Vec<String> =
                    record.fastq_ftp.split(";").map(|x| x.to_string()).collect();
                let ascp_url_vec: Vec<String> =
                    record.fastq_aspera.split(";").map(|x| x.to_string()).collect();
                let run_acc_: String = record.run_accession.to_string();
                for (md5, http_url, ascp_url) in izip!(md5_vec, http_url_vec, ascp_url_vec) {
                    let download_info = DownloadInfo {
                        orig_acc: self.orig_accession().to_string(),
                        run_acc: run_acc_.clone(),
                        http_url: "https://".to_string() + &http_url,
                        md5: md5,
                        ascp_url: "era-fasp@".to_string() + &ascp_url,
                    };
                    download_info_list.push(download_info);
                }
            }
            Some(download_info_list)
        } else {
            None
        }
    }
}

#[async_trait]
impl DownloadableAccession for Srx {
    fn orig_accession(&self) -> String {
        self.0.to_string()
    }
}
#[async_trait]
impl DownloadableAccession for Srr {
    fn orig_accession(&self) -> String {
        self.0.to_string()
    }
}
#[async_trait]
impl DownloadableAccession for Gsm {
    fn orig_accession(&self) -> String {
        self.0.to_string()
    }
    async fn get_downloadable_accession(&self) -> Option<String> {
        let client = reqwest::Client::new();
        let url = "https://www.ncbi.nlm.nih.gov/geo/query/acc.cgi";
        let orig_accession = self.orig_accession();
        let params = [
            ("acc", orig_accession.as_str()),
            ("targ", "self"),
            ("form", "xml"),
            ("view", "quick"),
        ];
        let resp_xml_text = client
            .get(url)
            .query(&params)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let doc = roxmltree::Document::parse(&resp_xml_text).unwrap();
        let root = doc.root_element();
        // one gsm should only have one Sample
        if let Some(sample_node) = root.children().find(|n| n.has_tag_name("Sample")) {
            // but may be multiple Relation
            for relation in sample_node
                .children()
                .filter(|n| n.has_tag_name("Relation"))
            {
                // find the one with attribute Type set to SRA
                if relation.attribute("type").unwrap() == "SRA" {
                    // get the attribute value of target
                    let target = relation.attribute("target").unwrap();
                    // target is in the format of "SRX0000001"
                    // use regex to extract it, include SRX in result
                    let srx = regex::Regex::new(r"SRX\d+")
                        .unwrap()
                        .find(&target)
                        .unwrap()
                        .as_str()
                        .to_string();
                    return Some(srx);
                }
            }
        }
        None
    }
}

#[async_trait]
impl DownloadableAccession for AccessionCodes {
    fn orig_accession(&self) -> String {
        match self {
            AccessionCodes::Srx(srx) => srx.0.to_string(),
            AccessionCodes::Srr(srr) => srr.0.to_string(),
            AccessionCodes::Gsm(gsm) => gsm.0.to_string(),
        }
    }
    async fn get_downloadable_accession(&self) -> Option<String> {
        match self {
            AccessionCodes::Srx(srx) => srx.get_downloadable_accession().await,
            AccessionCodes::Srr(srr) => srr.get_downloadable_accession().await,
            AccessionCodes::Gsm(gsm) => gsm.get_downloadable_accession().await,
        }
    }
    async fn get_download_info(&self) -> Option<Vec<DownloadInfo>> {
        match self {
            AccessionCodes::Srx(srx) => srx.get_download_info().await,
            AccessionCodes::Srr(srr) => srr.get_download_info().await,
            AccessionCodes::Gsm(gsm) => gsm.get_download_info().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_get_download_info_srr() {
        let run_acc = AccessionCodes::Srr(Srr("SRR000001".to_string()));
        let download_info_list = run_acc.get_download_info().await;
        println!("{:?}", download_info_list);
        let download_info_list = download_info_list.unwrap();
        assert_eq!(download_info_list.len(), 3);
        assert_eq!(download_info_list[0].orig_acc, "SRR000001");
        assert_eq!(download_info_list[0].run_acc, "SRR000001");
        assert_eq!(
            download_info_list[0].http_url,
            "ftp.sra.ebi.ac.uk/vol1/fastq/SRR000/SRR000001/SRR000001.fastq.gz"
        );
        assert_eq!(
            download_info_list[0].md5,
            "d656237bce7d2153e7d5326653fe950f"
        );
    }

    #[tokio::test]
    async fn test_get_download_info_srx() {
        let srx = AccessionCodes::Srx(Srx("SRX2243567".to_string()));
        let download_info_list = srx.get_download_info().await.unwrap();
        assert_eq!(download_info_list.len(), 1);
        assert_eq!(download_info_list[0].orig_acc, "SRX2243567");
        assert_eq!(download_info_list[0].run_acc, "SRR4421243");
        assert_eq!(
            download_info_list[0].http_url,
            "ftp.sra.ebi.ac.uk/vol1/fastq/SRR442/003/SRR4421243/SRR4421243.fastq.gz"
        );
        assert_eq!(
            download_info_list[0].md5,
            "325f82703836a7cc6b5fa84687376e86"
        );
    }

    #[tokio::test]
    async fn test_get_download_info_gsm() {
        let gsm = AccessionCodes::Gsm(Gsm("GSM2344754".to_string())); // in fact this is the same sample as above srx
        let download_info_list = gsm.get_download_info().await.unwrap();
        assert_eq!(download_info_list.len(), 1);
        assert_eq!(download_info_list[0].orig_acc, "GSM2344754");
        assert_eq!(download_info_list[0].run_acc, "SRR4421243");
        assert_eq!(
            download_info_list[0].http_url,
            "ftp.sra.ebi.ac.uk/vol1/fastq/SRR442/003/SRR4421243/SRR4421243.fastq.gz"
        );
        assert_eq!(
            download_info_list[0].md5,
            "325f82703836a7cc6b5fa84687376e86"
        );
    }
}

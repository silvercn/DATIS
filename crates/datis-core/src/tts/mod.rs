pub mod aws;
pub mod gcloud;

use std::fmt;
use std::str::FromStr;

#[derive(PartialEq, Clone)]
pub enum TextToSpeechProvider {
    GoogleCloud { voice: gcloud::VoiceKind },
    AmazonWebServices { voice: aws::VoiceKind },
}

#[derive(Clone)]
pub enum TextToSpeechConfig {
    GoogleCloud(gcloud::GoogleCloudConfig),
    AmazonWebServices(aws::AmazonWebServicesConfig),
}

impl Default for TextToSpeechProvider {
    fn default() -> Self {
        TextToSpeechProvider::GoogleCloud {
            voice: gcloud::VoiceKind::StandardC,
        }
    }
}

impl fmt::Debug for TextToSpeechProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            TextToSpeechProvider::GoogleCloud { voice } => {
                write!(f, "Google Cloud (Voice: {:?})", voice)
            }
            TextToSpeechProvider::AmazonWebServices { voice } => {
                write!(f, "Amazon Web Services (Voice: {:?})", voice)
            }
        }
    }
}

impl FromStr for TextToSpeechProvider {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<&str> = s.splitn(2, ':').collect();
        match v.as_slice() {
            &[prefix, voice] => match prefix {
                "GC" => {
                    return Ok(TextToSpeechProvider::GoogleCloud {
                        voice: gcloud::VoiceKind::from_str(voice)?,
                    })
                }
                "AWS" => {
                    return Ok(TextToSpeechProvider::AmazonWebServices {
                        voice: aws::VoiceKind::from_str(voice)?,
                    })
                }
                _ => {}
            },
            &[voice] if voice.len() > 0 => {
                return Ok(TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::from_str(voice)?,
                })
            }
            _ => {}
        }

        // fallback
        Ok(TextToSpeechProvider::GoogleCloud {
            voice: gcloud::VoiceKind::StandardC,
        })
    }
}

#[cfg(test)]
mod test {
    mod tts_provider_from_str {
        use std::str::FromStr;

        use crate::tts::{aws, gcloud, TextToSpeechProvider};

        #[test]
        fn fallback_on_empty_string() {
            assert_eq!(
                TextToSpeechProvider::from_str("").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardC
                }
            )
        }

        #[test]
        fn fallback_on_unknown_prefix() {
            assert_eq!(
                TextToSpeechProvider::from_str("UNK:foobar").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::StandardC
                }
            )
        }

        #[test]
        fn no_prefix_defaults_to_gcloud() {
            assert_eq!(
                TextToSpeechProvider::from_str("en-US-Wavenet-A").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::WavenetA
                }
            )
        }

        #[test]
        fn prefix_gc() {
            assert_eq!(
                TextToSpeechProvider::from_str("GC:en-US-Wavenet-B").unwrap(),
                TextToSpeechProvider::GoogleCloud {
                    voice: gcloud::VoiceKind::WavenetB
                }
            )
        }

        #[test]
        fn prefix_aws() {
            assert_eq!(
                TextToSpeechProvider::from_str("AWS:Brian").unwrap(),
                TextToSpeechProvider::AmazonWebServices {
                    voice: aws::VoiceKind::Brian
                }
            )
        }
    }
}
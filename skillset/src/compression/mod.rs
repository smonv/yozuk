use clap::Parser;
use itertools::iproduct;
use std::io::Read;
use yozuk_helper_encoding::EncodingPreprocessor;
use yozuk_helper_english::normalized_eq;
use yozuk_sdk::encoding::RawEncoding;
use yozuk_sdk::prelude::*;

mod algorithm;
use algorithm::ENTRIES;

pub const ENTRY: SkillEntry = SkillEntry {
    model_id: b"qX7I8WU4ACvSBY1zgiLWa",
    init: |_| {
        Skill::builder()
            .add_corpus(CompressionCorpus)
            .add_preprocessor(EncodingPreprocessor::new(RawEncoding::all()))
            .add_suggestions(CompressionSuggestions)
            .add_translator(CompressionTranslator)
            .set_command(CompressionCommand)
            .build()
    },
};

pub struct CompressionSuggestions;

impl Suggestions for CompressionSuggestions {
    fn suggestions(&self, _seed: u64, args: &[Token], _streams: &[InputStream]) -> Vec<String> {
        let inputs = args
            .iter()
            .filter(|arg| arg.tag == "input:data")
            .map(|arg| arg.as_str())
            .collect::<Vec<_>>();
        let joined = shell_words::join(if inputs.is_empty() {
            vec!["Hello World!"]
        } else {
            inputs
        });
        ENTRIES
            .iter()
            .filter_map(|entry| entry.keywords.iter().next())
            .map(|s| format!("{joined} to {s}"))
            .collect()
    }
}

pub struct CompressionCorpus;

impl Corpus for CompressionCorpus {
    fn training_data(&self) -> Vec<Vec<Token>> {
        let inputs = vec![
            "Hello World",
            "😍😗😋",
            "quick brown fox jumps over the lazy dog",
            "Veterinarian",
        ];
        iproduct!(
            inputs.clone(),
            ["as", "to", "in", "into"],
            ENTRIES.iter().flat_map(|entry| entry.keywords)
        )
        .map(|(data, prefix, alg)| {
            tk!([
                data; "input:data",
                prefix,
                *alg; "input:alg"
            ])
        })
        .chain(
            ENTRIES
                .iter()
                .flat_map(|entry| entry.keywords)
                .map(|alg| tk!([*alg; "input:alg"])),
        )
        .collect()
    }
}

pub struct CompressionTranslator;

impl Translator for CompressionTranslator {
    fn generate_command(&self, args: &[Token], _streams: &[InputStream]) -> Option<CommandArgs> {
        let input = args
            .iter()
            .filter(|arg| arg.tag == "input:data")
            .flat_map(|arg| ["--input", arg.as_str()]);

        let keywords = args
            .iter()
            .filter(|arg| arg.tag == "input:alg")
            .collect::<Vec<_>>();

        if !keywords.is_empty()
            && keywords.iter().all(|arg| {
                ENTRIES
                    .iter()
                    .any(|entry| normalized_eq(arg.as_str(), entry.keywords, 0))
            })
        {
            return Some(
                CommandArgs::new().add_args_iter(input).add_args_iter(
                    keywords
                        .iter()
                        .flat_map(|arg| ["--algorithm", arg.as_str()]),
                ),
            );
        }

        None
    }
}

pub struct CompressionCommand;

impl Command for CompressionCommand {
    fn run(
        &self,
        args: CommandArgs,
        streams: &mut [InputStream],
        _user: &UserContext,
    ) -> Result<Output, CommandError> {
        let args = Args::try_parse_from(args.args)?;

        let matched = ENTRIES
            .iter()
            .find(|entry| normalized_eq(&args.algorithm, entry.keywords, 0));

        let docs = Metadata::docs("https://docs.yozuk.com/docs/skills/compression/")?;

        if let Some(alg) = matched {
            let mut compressor = (alg.compressor)();
            if let [input, ..] = &args.input[..] {
                compressor.update(input.as_bytes());
            } else if let [stream, ..] = streams {
                let mut data = vec![0; 1024];
                while let Ok(len) = stream.read(&mut data) {
                    if len == 0 {
                        break;
                    }
                    compressor.update(&data[..len]);
                }
            }
            return Ok(Output::new()
                .set_title("Compression")
                .add_block(block::Data::new().set_data(compressor.finalize()))
                .add_metadata(docs));
        }

        Err(Output::new()
            .set_title("Compression")
            .add_metadata(docs)
            .add_block(
                block::Comment::new().set_text(format!("Unsupprted algorithm: {}", args.algorithm)),
            )
            .into())
    }

    fn priority(&self) -> i32 {
        -120
    }
}

#[derive(Parser)]
#[clap(trailing_var_arg = true)]
pub struct Args {
    #[clap(long)]
    pub algorithm: String,

    #[clap(short, long, multiple_occurrences(true))]
    pub input: Vec<String>,
}

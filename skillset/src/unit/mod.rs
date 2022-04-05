use bigdecimal::BigDecimal;
use clap::Parser;
use mediatype::media_type;
use num_bigint::BigInt;
use std::str::FromStr;
use yozuk_sdk::prelude::*;

mod conversion;
mod symbol;
mod unit;

use unit::*;

pub const ENTRY: SkillEntry = SkillEntry {
    model_id: b"86lRFe79o8JOiQCogjsXc",
    config_schema: None,
    init: |_, _| {
        Skill::builder()
            .add_corpus(UnitCorpus)
            .add_translator(UnitTranslator)
            .set_command(UnitCommand)
            .build()
    },
};

#[derive(Debug)]
pub struct UnitCorpus;

impl Corpus for UnitCorpus {
    fn training_data(&self) -> Vec<Vec<Token>> {
        symbol::ENTRIES
            .iter()
            .flat_map(|entry| {
                entry.symbols.iter().flat_map(|sym| {
                    entry
                        .prefixes
                        .iter()
                        .map(move |prefix| {
                            tk!([
                                "1.0"; "input:value",
                                format!("{}{}", prefix.to_string(), sym); "unit:keyword"
                            ])
                        })
                        .chain(Some(tk!([
                            "1.0"; "input:value",
                            sym.to_string(); "unit:keyword"
                        ])))
                })
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct UnitTranslator;

impl Translator for UnitTranslator {
    fn parse(&self, args: &[Token], _streams: &[InputStream]) -> Option<CommandArgs> {
        let values = args
            .iter()
            .filter(|arg| arg.tag == "input:value")
            .filter_map(|token| BigDecimal::from_str(token.as_utf8()).ok())
            .collect::<Vec<_>>();

        let units = args
            .iter()
            .filter(|arg| arg.tag == "unit:keyword")
            .filter(|arg| symbol::parse_symbol(arg.as_utf8()).is_some())
            .collect::<Vec<_>>();

        if let [value] = &values[..] {
            if let [unit] = units[..] {
                return Some(CommandArgs::new().add_args([
                    "--value".to_string(),
                    value.to_string(),
                    "--unit".to_string(),
                    unit.as_utf8().to_string(),
                ]));
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct UnitCommand;

impl Command for UnitCommand {
    fn run(&self, args: CommandArgs, _streams: &mut [InputStream]) -> Result<Output, CommandError> {
        let args = Args::try_parse_from(args.args)?;
        let value = BigDecimal::from_str(&args.value)?;
        let (prefix, base) = symbol::parse_symbol(&args.unit).unwrap();
        let base_unit = Unit {
            value: value.clone(),
            base,
            prefix,
        };
        let scale = BigDecimal::new(
            BigInt::from(1),
            prefix.map(|prefix| prefix.scale()).unwrap_or(0),
        );
        let converted = conversion::convert(&(value / scale), base);
        let converted = converted
            .into_iter()
            .filter(|unit| *unit != base_unit)
            .map(|unit| unit.to_string())
            .collect::<Vec<_>>();
        Ok(Output {
            module: "Unit Converter".into(),
            sections: vec![Section::new(
                format!("{} =\n{}", base_unit.to_string(), converted.join("\n")),
                media_type!(TEXT / PLAIN),
            )],
        })
    }

    fn priority(&self) -> i32 {
        -10
    }
}

#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    pub value: String,

    #[clap(short, long)]
    pub unit: String,
}
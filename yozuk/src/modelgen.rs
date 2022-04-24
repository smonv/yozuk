#![cfg(feature = "modelgen")]

use super::{skill, FeatureLabeler};
use anyhow::{bail, Result};
use crfsuite::{Algorithm, Attribute, GraphicalModel, Trainer};
use nanoid::nanoid;
use rayon::prelude::*;
use std::{
    collections::VecDeque,
    env,
    fs::{File, OpenOptions},
    io::{Read, Write},
    iter,
};
use yozuk_sdk::model::*;
use yozuk_sdk::prelude::*;

pub fn modelgen(env: &Environment) -> Result<ModelSet> {
    let mut keys = skill::SKILLS
        .iter()
        .map(|item| item.key.to_string())
        .collect::<Vec<_>>();
    keys.sort();

    let labelers = skill::SKILLS
        .par_iter()
        .flat_map(|item| {
            (item.entry.init)(env, &Default::default())
                .unwrap()
                .labelers
        })
        .collect::<Vec<_>>();

    let labeler = FeatureLabeler::new(&labelers);

    let dataset = skill::SKILLS
        .par_iter()
        .map(|item| TrainingData {
            key: item.key.to_string(),
            skills: vec![(item.entry.init)(env, &Default::default()).unwrap()],
            negative_skills: skill::SKILLS
                .par_iter()
                .filter(|neg| neg.key != item.key)
                .map(|neg| (neg.entry.init)(env, &Default::default()).unwrap())
                .collect(),
        })
        .filter_map(|item| learn(item, &labeler).ok())
        .collect::<Vec<_>>();

    let mut ranges = vec![0..0; keys.len()];
    let mut data = Vec::<u8>::new();

    for (key, mut item) in dataset {
        let index = keys.binary_search(&key).unwrap();
        ranges[index] = data.len()..data.len() + item.len();
        data.append(&mut item);
    }

    Ok(ModelSet::new(
        data,
        keys.into_iter().zip(ranges.into_iter()),
    ))
}

fn learn(item: TrainingData, labeler: &FeatureLabeler) -> Result<(String, Vec<u8>)> {
    let mut tr = Trainer::new(false);
    tr.select(Algorithm::LBFGS, GraphicalModel::CRF1D).unwrap();

    let seq = item
        .skills
        .iter()
        .map(|skill| (&skill.corpora, &skill.preprocessors))
        .flat_map(|(corpora, preps)| {
            corpora
                .iter()
                .flat_map(|corpus| corpus.training_data())
                .map(|tokens| {
                    preps
                        .iter()
                        .fold(tokens, |tokens, prep| prep.preprocess(tokens))
                })
        })
        .flat_map(generate_wordiness)
        .map(|data| {
            let (yseq, words): (Vec<_>, Vec<_>) = data
                .into_iter()
                .map(|token| (token.tag.clone(), token))
                .unzip();

            let xseq = labeler
                .label_features(&words)
                .into_iter()
                .map(|features| {
                    features
                        .into_iter()
                        .map(|feature| Attribute::new(feature.to_string(), 1.0))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            (xseq, yseq)
        })
        .collect::<Vec<_>>();

    if seq.is_empty() {
        bail!("no training data");
    }

    for (xseq, yseq) in &seq {
        tr.append(xseq, yseq, 0)?;
    }

    let seq = item
        .negative_skills
        .iter()
        .map(|skill| (&skill.corpora, &skill.preprocessors))
        .flat_map(|(corpora, preps)| {
            corpora
                .iter()
                .flat_map(|corpus| corpus.training_data())
                .map(|tokens| {
                    preps
                        .iter()
                        .fold(tokens, |tokens, prep| prep.preprocess(tokens))
                })
        })
        .flat_map(generate_wordiness)
        .map(|data| {
            let (yseq, words): (Vec<_>, Vec<_>) = data
                .into_iter()
                .map(|token| {
                    (
                        if token.tag == "-" {
                            "-".to_string()
                        } else {
                            "*".to_string()
                        },
                        token,
                    )
                })
                .unzip();

            let xseq = labeler
                .label_features(&words)
                .into_iter()
                .map(|features| {
                    features
                        .into_iter()
                        .map(|feature| Attribute::new(feature.to_string(), 0.01))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            (xseq, yseq)
        });

    for (xseq, yseq) in seq {
        tr.append(&xseq, &yseq, 0)?;
    }

    let filename = format!(
        "{}/{}.crfsuite",
        env::temp_dir().as_os_str().to_str().unwrap(),
        nanoid!()
    );

    tr.train(&filename, -1)?;
    let digest = skill::skills_digest();
    let mut file = OpenOptions::new().append(true).open(&filename).unwrap();
    file.write_all(&digest[..])?;
    file.flush()?;

    let mut file = File::open(&filename)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    Ok((item.key.to_string(), data))
}

struct TrainingData {
    key: String,
    skills: Vec<Skill>,
    negative_skills: Vec<Skill>,
}

fn generate_wordiness(data: Vec<Token>) -> impl Iterator<Item = Vec<Token>> {
    generate_wordiness_greetings(&data).chain(iter::once(data))
}

fn generate_wordiness_greetings(tokens: &[Token]) -> impl Iterator<Item = Vec<Token>> {
    let original = tokens.iter().cloned().collect::<VecDeque<_>>();
    let mut greetings = Vec::new();

    let mut data = original.clone();
    data.push_front(tk!("Yozuk,"));
    greetings.push(data.into_iter().collect::<Vec<_>>());

    let mut data = original;
    data.push_front(tk!("Yozuk,"));
    data.push_front(tk!("Hi"));
    greetings.push(data.into_iter().collect::<Vec<_>>());

    greetings.into_iter()
}

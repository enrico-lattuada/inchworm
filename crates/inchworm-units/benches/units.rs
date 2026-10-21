use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use inchworm_dimensions::{DimRegistry, Exp};
use inchworm_units::{Unit, UnitRegistry};

/// Same contents as the crate's `mks_registry` test fixture, built from the public API.
fn mks() -> UnitRegistry {
    let mut dims = DimRegistry::new("test-reg");
    let length = dims.add_base("length", None).unwrap();
    let mass = dims.add_base("mass", None).unwrap();
    let time = dims.add_base("time", None).unwrap();
    let temperature = dims.add_base("temperature", None).unwrap();
    let mut ureg = UnitRegistry::new("test-ureg", dims);
    ureg.add_prefix("kilo", "k", 1e3).unwrap();
    ureg.add_prefix("milli", "m", 1e-3).unwrap();
    ureg.add_prefix("centi", "c", 1e-2).unwrap();
    let kg = ureg.add_prefixed_base("kilo", "gram", "g", &mass).unwrap();
    let m = ureg.add_base("meter", "m", &length, true).unwrap();
    let s = ureg.add_base("second", "s", &time, true).unwrap();
    let kelvin = ureg.add_base("kelvin", "K", &temperature, true).unwrap();
    let m2 = m.pow(Exp::int(2)).unwrap();
    let s2 = s.pow(Exp::int(2)).unwrap();
    let joule_def = kg.try_mul(&m2).unwrap().try_div(&s2).unwrap();
    ureg.add_derived("joule", "J", &joule_def, 1.0, true)
        .unwrap();
    let delta_deg_fahrenheit = ureg
        .add_derived(
            "delta_deg_Fahrenheit",
            "delta_degF",
            &kelvin,
            5.0 / 9.0,
            false,
        )
        .unwrap();
    ureg.add_point("degree_Celsius", "degC", &kelvin, 273.15)
        .unwrap();
    ureg.add_point("degree_Fahrenheit", "degF", &delta_deg_fahrenheit, 459.67)
        .unwrap();
    ureg
}

fn parse(c: &mut Criterion) {
    let ureg = mks();
    c.bench_function("parse kg*m^2/s^2", |b| {
        b.iter(|| ureg.parse(black_box("kg*m^2/s^2")).unwrap())
    });
}

fn try_mul_chain(c: &mut Criterion) {
    let ureg = mks();
    let kg = ureg.get("kg").unwrap();
    let m = ureg.get("m").unwrap();
    let s = ureg.get("s").unwrap();
    c.bench_function("try_mul kg*m^2/s^2", |b| {
        b.iter(|| {
            kg.try_mul(&m)
                .unwrap()
                .try_mul(&m)
                .unwrap()
                .try_div(&s)
                .unwrap()
                .try_div(&s)
                .unwrap()
        })
    });
}

fn unit_clone(c: &mut Criterion) {
    let ureg = mks();
    let joule = ureg.get("joule").unwrap();
    c.bench_function("clone joule", |b| {
        b.iter(|| black_box(&joule).clone());
    });
}

fn to_base(c: &mut Criterion) {
    let ureg = mks();
    let Unit::Delta(unit) = ureg.parse("kJ/(kg*K)").unwrap() else {
        panic!("kJ/(kg*K) is a delta unit");
    };
    c.bench_function("to_base kJ/(kg*K)", |b| {
        b.iter(|| black_box(&unit).to_base().unwrap());
    });
}

criterion_group!(benches, parse, try_mul_chain, unit_clone, to_base);
criterion_main!(benches);

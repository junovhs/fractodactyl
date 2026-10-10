use fd_samples::*;

fn header(cols: ColumnSet) -> Header {
    Header {
        minor: MINOR,
        columns: cols,
        nx: 6,
        ny: 4,
        ss: 2,
        max_iter: 1 << 40,
        escape_radius: 65536.0,
        view: View {
            center_re: "-0.743643887037158704752191506114774".into(),
            center_im: "0.131825904205311970493132056385139".into(),
            width: "3e-30".into(),
            rotation: 0.25,
        },
        kernel: "test/1".into(),
    }
}

fn path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("fd-samples-{}-{name}.fds", std::process::id()))
}

#[test]
fn every_column_round_trips_bit_exactly() {
    let all = ColumnSet(ColumnSet::KNOWN);
    let h = header(all);
    let mut s = Samples::alloc(h.count(), all);
    for i in 0..h.count() {
        s.class[i] = Class::new([Kind::Escaped, Kind::Interior, Kind::Unresolved][i % 3], Evidence::Heuristic);
        s.nu.as_mut().unwrap()[i] = 1e9 + i as f64 / 7.0;
        s.de.as_mut().unwrap()[i] = i as f32 * 0.5;
        s.normal.as_mut().unwrap()[i] = (i * 4099) as u16;
        s.bound.as_mut().unwrap()[i] = 1e-6;
    }
    let p = path("all");
    write(std::fs::File::create(&p).unwrap(), &h, &s).unwrap();
    let mut r = Reader::open(&p).unwrap();
    assert_eq!(r.header, h);
    assert_eq!(r.read(all).unwrap(), s);
    std::fs::remove_file(p).unwrap();
}

#[test]
fn reads_only_requested_columns() {
    let set = ColumnSet::of(&[Column::Class, Column::Nu, Column::Normal]);
    let h = header(set);
    let s = Samples::alloc(h.count(), set);
    let p = path("some");
    write(std::fs::File::create(&p).unwrap(), &h, &s).unwrap();
    let mut r = Reader::open(&p).unwrap();
    let got = r.read(ColumnSet::of(&[Column::Normal])).unwrap();
    assert!(got.nu.is_none() && got.normal.is_some());
    assert!(r.read(ColumnSet::of(&[Column::De])).is_err());
    std::fs::remove_file(p).unwrap();
}

#[test]
fn mismatched_header_is_refused() {
    let h = header(ColumnSet::of(&[Column::Class, Column::Nu]));
    let s = Samples::alloc(h.count(), ColumnSet::of(&[Column::Class]));
    assert!(write(Vec::new(), &h, &s).is_err());
}

#[test]
fn normal_angle_quantisation() {
    for k in 0..64 {
        let t = k as f64 * std::f64::consts::TAU / 64.0 - 3.0;
        let (x, y) = Samples::unit(Samples::angle(t.cos(), t.sin()));
        assert!((x as f64 - t.cos()).abs() < 1e-4 && (y as f64 - t.sin()).abs() < 1e-4);
    }
}

#[test]
fn huge_dimensions_are_refused_without_allocating() {
    let mut h = header(ColumnSet(ColumnSet::KNOWN));
    h.nx = 1 << 31;
    h.ny = 1 << 31;
    h.ss = 1;
    let p = path("huge");
    std::fs::write(&p, h.encode().unwrap()).unwrap();
    let err = Reader::open(&p).err().expect("huge sample file must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    std::fs::remove_file(p).unwrap();
}

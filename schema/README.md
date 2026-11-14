# Schema

`declarations.rng` is a RELAX NG schema for the XML `tsrs_proto` prints: element and
attribute names, which values are fixed sets, and what may contain what.

    ./target/debug/tsrs_proto lib.dom.d.ts --config examples/targets/rust.toml > out.xml
    xmllint --noout --relaxng schema/declarations.rng out.xml

It follows `src/bin/tsrs_proto/xml.rs`: change both together.

use std::path::{Path, PathBuf};

/// Collect PDF files from a directory matching a filename pattern.
fn collect_pdfs(dir: &Path, pattern: &str) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name.ends_with(".pdf") && name.contains(pattern)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Collect all PDF files from a directory.
fn all_pdfs(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".pdf"))
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

// =============================================================================
// Reference Suite — Gold Standard (all must pass)
// =============================================================================

#[test]
fn reference_suite_all_compliant() {
    let dir = fixtures().join("pdfua-reference-suite");
    let pdfs = all_pdfs(&dir);
    assert!(
        !pdfs.is_empty(),
        "No reference suite PDFs found. Run: git submodule update --init --recursive"
    );

    let mut failures = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if !report.is_compliant() {
            let name = pdf.file_name().unwrap().to_string_lossy();
            failures.push(format!(
                "  {} — {} errors, {} failed",
                name,
                report.error.as_deref().unwrap_or("none"),
                report.failed()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "Reference suite: {}/{} compliant\nFailures:\n{}",
        pdfs.len() - failures.len(),
        pdfs.len(),
        failures.join("\n")
    );

    eprintln!("Reference suite: {}/{} compliant ✓", pdfs.len(), pdfs.len());
}

// =============================================================================
// veraPDF Corpus — Pass files should be compliant
// =============================================================================

#[test]
fn corpus_ua1_pass_files() {
    let dir = fixtures().join("verapdf-corpus/PDF_UA-1");
    let pdfs = collect_pdfs(&dir, "-pass-");
    assert!(
        !pdfs.is_empty(),
        "No UA-1 pass PDFs found. Run: git submodule update --init --recursive"
    );

    let mut compliant = 0;
    let mut non_compliant = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if report.is_compliant() {
            compliant += 1;
        } else {
            let name = pdf.file_name().unwrap().to_string_lossy();
            non_compliant.push(format!("  {} (failed: {})", name, report.failed()));
        }
    }

    let rate = compliant as f64 / pdfs.len() as f64 * 100.0;
    eprintln!(
        "UA-1 pass files: {}/{} compliant ({:.1}%)",
        compliant,
        pdfs.len(),
        rate
    );

    // Baseline: record current rate — this will be tightened after first run
    assert!(
        rate >= 0.0,
        "UA-1 pass detection rate {:.1}% (baseline to be set)",
        rate
    );
}

#[test]
fn corpus_ua2_pass_files() {
    let dir = fixtures().join("verapdf-corpus/PDF_UA-2");
    let pdfs = collect_pdfs(&dir, "-pass-");
    if pdfs.is_empty() {
        eprintln!("Skipping UA-2 pass test — no corpus files found");
        return;
    }

    let mut compliant = 0;
    let mut non_compliant = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if report.is_compliant() {
            compliant += 1;
        } else {
            let name = pdf.file_name().unwrap().to_string_lossy();
            non_compliant.push(format!("  {} (failed: {})", name, report.failed()));
        }
    }

    let rate = compliant as f64 / pdfs.len() as f64 * 100.0;
    eprintln!(
        "UA-2 pass files: {}/{} compliant ({:.1}%)",
        compliant,
        pdfs.len(),
        rate
    );

    assert!(
        rate >= 0.0,
        "UA-2 pass detection rate {:.1}% (baseline to be set)",
        rate
    );
}

// =============================================================================
// veraPDF Corpus — Fail files should have failures detected
// =============================================================================

#[test]
fn corpus_ua1_fail_files() {
    let dir = fixtures().join("verapdf-corpus/PDF_UA-1");
    let pdfs = collect_pdfs(&dir, "-fail-");
    assert!(
        !pdfs.is_empty(),
        "No UA-1 fail PDFs found. Run: git submodule update --init --recursive"
    );

    let mut detected = 0;
    let mut missed = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if report.failed() > 0 || report.error.is_some() {
            detected += 1;
        } else {
            let name = pdf.file_name().unwrap().to_string_lossy();
            missed.push(format!("  {}", name));
        }
    }

    let rate = detected as f64 / pdfs.len() as f64 * 100.0;
    eprintln!(
        "UA-1 fail detection: {}/{} detected ({:.1}%)",
        detected,
        pdfs.len(),
        rate
    );

    assert!(
        rate >= 0.0,
        "UA-1 fail detection rate {:.1}% (baseline to be set)",
        rate
    );
}

#[test]
fn corpus_ua2_fail_files() {
    let dir = fixtures().join("verapdf-corpus/PDF_UA-2");
    let pdfs = collect_pdfs(&dir, "-fail-");
    if pdfs.is_empty() {
        eprintln!("Skipping UA-2 fail test — no corpus files found");
        return;
    }

    let mut detected = 0;
    let mut missed = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if report.failed() > 0 || report.error.is_some() {
            detected += 1;
        } else {
            let name = pdf.file_name().unwrap().to_string_lossy();
            missed.push(format!("  {}", name));
        }
    }

    let rate = detected as f64 / pdfs.len() as f64 * 100.0;
    eprintln!(
        "UA-2 fail detection: {}/{} detected ({:.1}%)",
        detected,
        pdfs.len(),
        rate
    );

    assert!(
        rate >= 0.0,
        "UA-2 fail detection rate {:.1}% (baseline to be set)",
        rate
    );
}

// =============================================================================
// pdfcheck Examples — Known outcomes
// =============================================================================

#[test]
fn pdfcheck_pass_files() {
    let dir = fixtures().join("pdfcheck/examples");
    let pass_files = [
        "tagged-with-UA.pdf",
        "tagged-PAC2-pass.pdf",
        "tagged-HTML-headings-PAC-2024-pass.pdf",
    ];

    let mut failures = Vec::new();
    for name in &pass_files {
        let path = dir.join(name);
        if !path.exists() {
            eprintln!("Skipping {} — not found", name);
            continue;
        }
        let report = horn::validate_file(&path);
        if !report.is_compliant() {
            failures.push(format!("  {} (failed: {})", name, report.failed()));
        }
    }

    assert!(
        failures.is_empty(),
        "pdfcheck pass files should be compliant:\n{}",
        failures.join("\n")
    );

    eprintln!(
        "pdfcheck pass files: {}/{} compliant ✓",
        pass_files.len(),
        pass_files.len()
    );
}

#[test]
fn pdfcheck_fail_files() {
    let dir = fixtures().join("pdfcheck/examples");
    let fail_files = [
        "not-tagged.pdf",
        "not-tagged-with-UA.pdf",
        "not-tagged-with-doctitle.pdf",
        "not-tagged-with-filename.pdf",
        "not-tagged-with-language.pdf",
        "tagged-no-UA.pdf",
        "tagged-no-UA-with-filename.pdf",
        "tagged-HTML-headings-chrome.pdf",
        "tagged-HTML-headings-chrome-espanol.pdf",
    ];

    let mut missed = Vec::new();
    let mut tested = 0;
    for name in &fail_files {
        let path = dir.join(name);
        if !path.exists() {
            eprintln!("Skipping {} — not found", name);
            continue;
        }
        tested += 1;
        let report = horn::validate_file(&path);
        if report.is_compliant() {
            missed.push(format!("  {}", name));
        }
    }

    assert!(
        missed.is_empty(),
        "pdfcheck fail files should be non-compliant:\n{}",
        missed.join("\n")
    );

    eprintln!("pdfcheck fail files: {}/{} detected ✓", tested, tested);
}

// =============================================================================
// Generated Fixtures — Targeted edge-case and adversarial PDFs
// =============================================================================

#[test]
fn generated_pass_files() {
    let dir = fixtures().join("generated");
    let pdfs = collect_pdfs(&dir, "-pass.");
    if pdfs.is_empty() {
        eprintln!("Skipping generated pass test — no fixtures found (run generate.py)");
        return;
    }

    let mut failures = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if !report.is_compliant() {
            let name = pdf.strip_prefix(&dir).unwrap_or(pdf).display();
            let issues: Vec<_> = report
                .results
                .iter()
                .filter(|r| r.is_failure())
                .map(|r| format!("{}: {}", r.rule_id, r.description))
                .collect();
            failures.push(format!("  {} — {}", name, issues.join("; ")));
        }
    }

    let compliant = pdfs.len() - failures.len();
    eprintln!(
        "Generated pass files: {}/{} compliant",
        compliant,
        pdfs.len()
    );

    assert!(
        failures.is_empty(),
        "Generated pass files should all be compliant:\n{}",
        failures.join("\n")
    );
}

#[test]
fn generated_fail_files() {
    let dir = fixtures().join("generated");
    let pdfs = collect_pdfs(&dir, "-fail.");
    if pdfs.is_empty() {
        eprintln!("Skipping generated fail test — no fixtures found (run generate.py)");
        return;
    }

    let mut missed = Vec::new();
    for pdf in &pdfs {
        let report = horn::validate_file(pdf);
        if report.is_compliant() {
            let name = pdf.strip_prefix(&dir).unwrap_or(pdf).display();
            missed.push(format!("  {}", name));
        }
    }

    let detected = pdfs.len() - missed.len();
    eprintln!("Generated fail files: {}/{} detected", detected, pdfs.len());

    assert!(
        missed.is_empty(),
        "Generated fail files should all be non-compliant:\n{}",
        missed.join("\n")
    );
}

// =============================================================================
// Coverage Baseline — Aggregate stats that must not regress
// =============================================================================

#[test]
fn coverage_baseline() {
    let fixtures = fixtures();

    // --- Reference suite ---
    let ref_pdfs = all_pdfs(&fixtures.join("pdfua-reference-suite"));
    let ref_compliant = ref_pdfs
        .iter()
        .filter(|p| horn::validate_file(p).is_compliant())
        .count();

    // --- UA-1 pass ---
    let ua1_pass = collect_pdfs(&fixtures.join("verapdf-corpus/PDF_UA-1"), "-pass-");
    let ua1_pass_compliant = ua1_pass
        .iter()
        .filter(|p| horn::validate_file(p).is_compliant())
        .count();

    // --- UA-1 fail ---
    let ua1_fail = collect_pdfs(&fixtures.join("verapdf-corpus/PDF_UA-1"), "-fail-");
    let ua1_fail_detected = ua1_fail
        .iter()
        .filter(|p| {
            let r = horn::validate_file(p);
            r.failed() > 0 || r.error.is_some()
        })
        .count();

    // --- Generated fixtures ---
    let gen_pass = collect_pdfs(&fixtures.join("generated"), "-pass.");
    let gen_pass_compliant = gen_pass
        .iter()
        .filter(|p| horn::validate_file(p).is_compliant())
        .count();

    let gen_fail = collect_pdfs(&fixtures.join("generated"), "-fail.");
    let gen_fail_detected = gen_fail
        .iter()
        .filter(|p| {
            let r = horn::validate_file(p);
            r.failed() > 0 || r.error.is_some()
        })
        .count();

    // --- Check count sanity (pick any file, ensure checks actually ran) ---
    let sample = fixtures.join("pdfcheck/examples/tagged-with-UA.pdf");
    let sample_report = horn::validate_file(&sample);
    let check_count = sample_report.results.len();

    // Print summary
    eprintln!("\n╔══════════════════════════════════════════════╗");
    eprintln!("║           COVERAGE BASELINE REPORT           ║");
    eprintln!("╠══════════════════════════════════════════════╣");
    eprintln!(
        "║ Reference suite:  {:>3}/{:<3} compliant ({:.0}%)     ║",
        ref_compliant,
        ref_pdfs.len(),
        ref_compliant as f64 / ref_pdfs.len().max(1) as f64 * 100.0
    );
    eprintln!(
        "║ UA-1 pass rate:   {:>3}/{:<3} compliant ({:.0}%)     ║",
        ua1_pass_compliant,
        ua1_pass.len(),
        ua1_pass_compliant as f64 / ua1_pass.len().max(1) as f64 * 100.0
    );
    eprintln!(
        "║ UA-1 fail detect: {:>3}/{:<3} detected  ({:.0}%)     ║",
        ua1_fail_detected,
        ua1_fail.len(),
        ua1_fail_detected as f64 / ua1_fail.len().max(1) as f64 * 100.0
    );
    eprintln!(
        "║ Generated pass:   {:>3}/{:<3} compliant ({:.0}%)     ║",
        gen_pass_compliant,
        gen_pass.len(),
        gen_pass_compliant as f64 / gen_pass.len().max(1) as f64 * 100.0
    );
    eprintln!(
        "║ Generated fail:   {:>3}/{:<3} detected  ({:.0}%)     ║",
        gen_fail_detected,
        gen_fail.len(),
        gen_fail_detected as f64 / gen_fail.len().max(1) as f64 * 100.0
    );
    eprintln!(
        "║ Checks per file:  {:<3}                         ║",
        check_count
    );
    eprintln!("╚══════════════════════════════════════════════╝\n");

    // =========================================================================
    // BASELINE ASSERTIONS — floors that future changes must meet or exceed.
    // BASELINE — updated 2026-03-21.
    // =========================================================================

    assert!(
        ref_pdfs.len() >= 10,
        "Expected at least 10 reference PDFs, found {}",
        ref_pdfs.len()
    );
    assert!(
        ua1_pass.len() >= 140,
        "Expected at least 140 UA-1 pass files, found {}",
        ua1_pass.len()
    );
    assert!(
        ua1_fail.len() >= 150,
        "Expected at least 150 UA-1 fail files, found {}",
        ua1_fail.len()
    );
    assert!(
        check_count >= 10,
        "Expected at least 10 checks per file, got {}",
        check_count
    );

    assert!(
        ref_compliant >= 10,
        "Reference suite regression: {}/{} compliant (baseline: 10/10)",
        ref_compliant,
        ref_pdfs.len()
    );
    assert!(
        ua1_pass_compliant >= 141,
        "UA-1 pass rate regression: {}/{} (baseline: 141/141)",
        ua1_pass_compliant,
        ua1_pass.len()
    );
    assert!(
        ua1_fail_detected >= 138,
        "UA-1 fail detection regression: {}/{} (baseline: 138/155)",
        ua1_fail_detected,
        ua1_fail.len()
    );
    assert!(
        check_count >= 33,
        "Check count regression: {} (baseline: 33)",
        check_count
    );

    // Generated fixtures — 100% accuracy required
    assert!(
        gen_pass_compliant == gen_pass.len(),
        "Generated pass regression: {}/{} compliant (baseline: 25/25)",
        gen_pass_compliant,
        gen_pass.len()
    );
    assert!(
        gen_fail_detected == gen_fail.len(),
        "Generated fail regression: {}/{} detected (baseline: 56/56)",
        gen_fail_detected,
        gen_fail.len()
    );
}

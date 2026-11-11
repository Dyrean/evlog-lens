mod app;
mod file_picker;
mod importer;
mod log;
mod query;
mod shell;
mod storage;
mod ui;
mod workspace;

fn main() {
    shell::run();
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

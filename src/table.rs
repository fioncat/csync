use pad::PadStr;

pub struct Table {
    ncol: usize,
    rows: Vec<Vec<String>>,
    headless: bool,
}

impl Table {
    pub fn with_capacity(size: usize, headless: bool) -> Table {
        Table {
            ncol: 0,
            rows: Vec::with_capacity(size),
            headless,
        }
    }

    pub fn add(&mut self, row: Vec<String>) {
        if self.ncol == 0 {
            self.ncol = row.len();
            if self.headless {
                return;
            }
        } else if row.len() != self.ncol {
            panic!("unexpected row len");
        }
        self.rows.push(row);
    }

    pub fn show(self) {
        let mut pads = Vec::with_capacity(self.ncol);
        for coli in 0..self.ncol {
            let mut max_size: usize = 0;
            for row in self.rows.iter() {
                let cell = row.get(coli).unwrap();
                let size = console::measure_text_width(cell);
                if size > max_size {
                    max_size = size
                }
            }
            pads.push(max_size);
        }

        let mut split = String::from("+");
        for pad in pads.iter() {
            for _ in 0..*pad + 2 {
                split.push('-');
            }
            split.push('+');
        }

        for (rowi, row) in self.rows.into_iter().enumerate() {
            if rowi == 0 {
                eprintln!("{split}");
            }
            eprint!("|");
            for (coli, cell) in row.into_iter().enumerate() {
                let pad = pads[coli];
                let text = cell.pad_to_width_with_alignment(pad, pad::Alignment::Left);
                eprint!(" {text} |");
            }
            eprintln!();

            if !self.headless && rowi == 0 {
                eprintln!("{split}");
            }
        }

        eprintln!("{split}");
    }
}

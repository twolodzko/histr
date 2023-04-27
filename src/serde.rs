extern crate serde;

use crate::bin::Bin;
use crate::hist::StreamHist;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{Read, Write};
use std::iter::zip;

// See: https://rust-by-example-ext.com/serde/json.html
#[derive(Serialize, Deserialize, Debug)]
struct HistJson {
    means: Vec<f64>,
    counts: Vec<u64>,
    min: Option<f64>,
    max: Option<f64>,
}

impl StreamHist {
    /// Read the histogram from a JSON string.
    ///
    /// The JSON needs to contain two numeric arrays for `"means"` and `"counts"`, and optional fields for
    /// `min` and `max` (can be `null` as in the example in [`StreamHist::to_json`]).
    /// When `min` and `max` are not given, they are set to smallest and largest bin means respectively.
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    /// use streamhist::Bin;
    ///
    /// assert_eq!(
    ///     StreamHist::from_json(
    ///         r#"{
    ///             "means":  [3,1,2],
    ///             "counts": [2,3,4]
    ///         }"#
    ///     ),
    ///     StreamHist::from(vec![Bin::new(1.0, 3), Bin::new(2.0, 4), Bin::new(3.0, 2)])
    /// );
    /// ```
    pub fn from_json(json: &str) -> Self {
        let h: HistJson = serde_json::from_str(json).unwrap();
        StreamHist::from(h)
    }

    /// Transform the histogram to a JSON string.
    ///
    /// See [`StreamHist::from_json`] for more details.
    ///
    /// # Examples
    ///
    /// ```
    /// use streamhist::StreamHist;
    ///
    /// let hist = StreamHist::default();
    /// assert_eq!(hist.to_json(), r#"{"means":[],"counts":[],"min":null,"max":null}"#);
    /// ```
    pub fn to_json(&self) -> String {
        let h = HistJson::from(self);
        serde_json::to_string(&h).unwrap()
    }

    /// Read histogram from JSON using a reader.
    ///
    /// See [`StreamHist::from_json`] for more details.
    pub fn read_json<R>(reader: R) -> Result<Self, Box<dyn Error>>
    where
        R: Read,
    {
        let json: HistJson = serde_json::from_reader(reader).map_err(Box::new)?;
        Ok(StreamHist::from(json))
    }

    /// Write histogram to JSON using a writer.
    ///
    /// See [`StreamHist::from_json`] for more details.
    pub fn write_json<W>(&self, writer: &mut W) -> Result<(), Box<dyn Error>>
    where
        W: Write,
    {
        write!(writer, "{}", self.to_json()).map_err(Box::new)?;
        Ok(())
    }

    /// Read histogram from a [MessagePack] format using a reader.
    ///
    /// [MessagePack]: https://msgpack.org/
    ///
    /// # Examples
    /// ```
    /// extern crate tempdir;
    /// use std::fs::File;
    /// use tempdir::TempDir;
    /// use streamhist::StreamHist;
    ///
    /// // initialize a temporary directory
    /// let temp_dir = TempDir::new("example").unwrap();
    /// let file_path = temp_dir.path().join("hist.msgpack");
    /// // create a file in it
    /// let file_to_write = &mut File::create(file_path.clone()).unwrap();
    ///
    /// let orig_hist = StreamHist::from(vec![2.0, 5.0, 1.0, 3.0, 4.0, 1.0, 2.5]);
    /// // write the histogram to the file
    /// orig_hist.write_msgpack(file_to_write)
    ///     .expect("failed writing the file");
    ///
    /// // open the file again and read from it
    /// let file_to_read = &mut File::open(file_path).unwrap();
    /// let read_hist = StreamHist::read_msgpack(file_to_read).expect("failed reading the file");
    ///
    /// assert_eq!(orig_hist, read_hist);
    /// ```
    pub fn read_msgpack<R>(reader: R) -> Result<Self, Box<dyn Error>>
    where
        R: Read,
    {
        let hist = rmp_serde::decode::from_read(reader).map_err(Box::new)?;
        Ok(hist)
    }

    /// Write histogram to [MessagePack] format using a writer.
    ///
    /// [MessagePack]: https://msgpack.org/
    pub fn write_msgpack<W>(&self, writer: &mut W) -> Result<(), Box<dyn Error>>
    where
        W: Write,
    {
        rmp_serde::encode::write(writer, self).map_err(Box::new)?;
        Ok(())
    }
}

impl From<HistJson> for StreamHist {
    fn from(h: HistJson) -> Self {
        let mut bins: Vec<Bin> = zip(h.means, h.counts)
            .map(|(m, c)| Bin::new(m, c))
            .collect();
        bins.sort();
        let mut hist = StreamHist::from(bins);
        if let Some(min) = h.min {
            hist.min = min;
        }
        if let Some(max) = h.max {
            hist.max = max;
        }
        hist
    }
}

impl From<&StreamHist> for HistJson {
    fn from(h: &StreamHist) -> Self {
        let (means, counts) = h.iter().map(|bin| bin.into()).unzip();
        HistJson {
            means,
            counts,
            min: if h.min.is_nan() { None } else { Some(h.min) },
            max: if h.max.is_nan() { None } else { Some(h.max) },
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use crate::bin::Bin;
    use crate::hist::StreamHist;
    use std::fs::File;
    use tempdir::TempDir;

    #[test]
    fn from_json() {
        assert_eq!(
            StreamHist::from_json("{\"means\":[],\"counts\":[]}"),
            StreamHist::default()
        );
        assert_eq!(
            StreamHist::from_json("{\"means\":[],\"counts\":[],\"min\":null,\"max\":null}"),
            StreamHist::default()
        );

        assert_eq!(
            StreamHist::from_json(
                "{
                    \"means\":  [3,1,2],
                    \"counts\": [2,3,4]
                }"
            ),
            StreamHist::from(vec![Bin::new(1.0, 3), Bin::new(2.0, 4), Bin::new(3.0, 2)])
        );

        assert_eq!(
            StreamHist::from_json(
                "{
                    \"means\":  [3,1,2],
                    \"counts\": [2,3,4],
                    \"min\": 0,
                    \"max\": 5
                }"
            ),
            StreamHist {
                bins: vec![Bin::new(1.0, 3), Bin::new(2.0, 4), Bin::new(3.0, 2)],
                min: 0.0,
                max: 5.0,
                size: 3,
            }
        );
    }

    #[test]
    fn to_json() {
        assert_eq!(
            StreamHist::with_capacity(5).to_json(),
            "{\"means\":[],\"counts\":[],\"min\":null,\"max\":null}"
        );
        assert_eq!(
            StreamHist::from(vec![Bin::new(1.0, 3), Bin::new(2.0, 4), Bin::new(3.0, 2)]).to_json(),
            String::from("{\"means\":[1.0,2.0,3.0],\"counts\":[3,4,2],\"min\":1.0,\"max\":3.0}")
        );
    }

    #[test]
    fn write_read_json() {
        let temp_dir = TempDir::new("tests").unwrap();
        let file_path = temp_dir.path().join("hist.json");
        let file_to_write = &mut File::create(file_path.clone()).unwrap();
        let hist = StreamHist::from(vec![2.0, 5.0, 1.0, 3.0, 4.0, 1.0, 2.5]);

        hist.write_json(file_to_write)
            .expect("failed writing the file");

        let file_to_read = &mut File::open(file_path).unwrap();
        assert_eq!(
            hist,
            StreamHist::read_json(file_to_read).expect("failed reading the file")
        );
    }

    #[test]
    fn write_read_msgpack() {
        let temp_dir = TempDir::new("tests").unwrap();
        let file_path = temp_dir.path().join("hist.msgpack");
        let file_to_write = &mut File::create(file_path.clone()).unwrap();
        let hist = StreamHist::from(vec![2.0, 5.0, 1.0, 3.0, 4.0, 1.0, 2.5]);

        hist.write_msgpack(file_to_write)
            .expect("failed writing the file");

        let file_to_read = &mut File::open(file_path).unwrap();
        assert_eq!(
            hist,
            StreamHist::read_msgpack(file_to_read).expect("failed reading the file")
        );
    }
}

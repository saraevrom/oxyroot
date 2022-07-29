use crate::factory_all_for_register_impl;
use crate::rbase;
use crate::rbytes::rbuffer::RBuffer;
use crate::rbytes::Unmarshaler;
use crate::rcont::objarray::ObjArray;
use crate::riofs::file::{RootFileReader, RootFileStreamerInfoContext};
use crate::root::traits::Named;
use crate::root::traits::Object;
use crate::rtree::branch::Branch;
use crate::rvers;
use anyhow::{bail, ensure};
use log::trace;

#[derive(Default)]
pub struct Clusters {
    ranges: Vec<i64>,
    sizes: Vec<i64>,
}

// pub struct TioFeatures {
//     val:
// }

#[derive(Default, Debug)]
pub struct TioFeatures(u8);

impl Unmarshaler for TioFeatures {
    fn unmarshal(&mut self, r: &mut RBuffer) -> anyhow::Result<()> {
        let hdr = r.read_header(self.class())?;
        ensure!(
            hdr.vers <= rvers::ROOT_IOFEATURES,
            "rtree: invalid {} version={} > {}",
            self.class(),
            hdr.vers,
            rvers::ROOT_IOFEATURES
        );

        let mut buf = [0_u8; 4];
        r.read_array_u8(&mut buf[..1])?;

        self.0 = if buf[0] != 0 {
            r.read_array_u8(&mut buf[1..])?;
            r.read_u8()?
        } else {
            0
        };

        r.check_header(&hdr)?;

        Ok(())

        // trace!("buf = {:?}", buf);
        //
        // todo!()
    }
}

factory_all_for_register_impl!(TioFeatures, "TIOFeatures");
/// Rust equivalent of [`TTree`](https://root.cern/doc/master/classTTree.html)
///
/// Mainly used to retrieve [crate::Branch] ant iterate over them
#[derive(Default)]
pub struct Tree {
    rvers: i16,
    named: rbase::Named,
    attline: rbase::AttLine,
    attfill: rbase::AttFill,
    attmarker: rbase::AttMarker,

    /// Number of entries
    entries: i64,
    /// Total number of bytes in all branches before compression
    tot_bytes: i64,
    /// Total number of bytes in all branches after  compression
    zip_bytes: i64,
    /// number of autosaved bytes
    saved_bytes: i64,
    /// number of auto-flushed bytes
    flushed_bytes: i64,

    /// tree weight
    weight: f64,
    /// timer interval in milliseconds
    timer_interval: i32,
    /// number of runs before prompting in Scan
    scan_field: i32,
    /// update frequency for entry-loop
    update: i32,
    /// initial length of the entry offset table in the basket buffers
    default_entry_offset_len: i32,
    /// maximum number of entries in case of circular buffers
    max_entries: i64,
    /// maximum number of entries to process
    max_entry_loop: i64,
    /// maximum total size of buffers kept in memory
    max_virtual_size: i64,
    /// auto_save tree when auto_save entries written
    auto_save: i64,
    /// auto_flush tree when auto_flush entries written
    auto_flush: i64,
    /// number of entries to estimate histogram limits
    estimate: i64,

    clusters: Clusters,
    iobits: TioFeatures,

    branches: Vec<Branch>,

    reader: Option<RootFileReader>,
    sinfos: Option<RootFileStreamerInfoContext>,
}

impl Tree {
    pub(crate) fn set_reader(&mut self, reader: Option<RootFileReader>) {
        if let Some(r) = &reader {
            for b in self.branches.iter_mut() {
                b.set_reader(Some(r.clone()));
            }
            self.reader = reader;
        }
    }

    pub(crate) fn set_streamer_info(&mut self, sinfos: RootFileStreamerInfoContext) {
        for b in self.branches.iter_mut() {
            b.set_streamer_info(sinfos.clone());
        }
        self.sinfos = Some(sinfos);
    }

    /// Get a branch from this tree
    pub fn branch(&self, name: &str) -> Option<&Branch> {
        for b in self.branches.iter() {
            if b.name() == name {
                return Some(b);
            }

            if let Some(bb) = b.branch(name) {
                return Some(bb);
            }
        }
        None
    }

    /// Get iterator over top-level branches
    pub fn branches(&self) -> impl Iterator<Item = &Branch> {
        self.branches.iter()
    }

    /// Number of entries in the TTree, as reported by fEntries.
    pub fn entries(&self) -> i64 {
        self.entries
    }

    /// Get all (recursively) branches in this tree
    pub fn branches_r(&self) -> Vec<&Branch> {
        let mut v = Vec::new();

        for b in self.branches() {
            trace!("ADD {:?}", b);
            v.push(b);
            for bb in b.branches_r() {
                v.push(bb);
            }
        }

        v
    }

    /// Display branches in this tree
    ///
    /// Provide name, C++ type and a possible Rust interpretation.
    ///
    /// Example:
    /// ```ignore
    /// name                           | typename                       | interpretation                
    /// -------------------------------+-------------------------------+-------------------------------
    /// string                         | string                         | String                        
    /// vector_vector_int32            | vector<vector<int32_t>>        | Vec<Vec<i32>>                      
    /// vector_int32                   | vector<int32_t>                | Vec<i32>                      
    /// vector_string                  | vector<string>                 | Vec<String>                   
    /// three                          | char*                          | String
    ///```
    ///
    /// In this example, last branch can be read with:
    /// ```ignore    
    /// let three = tree
    ///         .branch("three")
    ///         .unwrap()
    ///         .as_iter::<String>()
    ///         .collect::<Vec<_>>();
    /// ```

    pub fn show(&self) {
        // const TYPE_NAME_SIZE:usize = 30;
        println!(
            "{:<30} | {:<30} | {:<30}",
            "name", "typename", "interpretation"
        );
        let s: String = ['-'; 31].iter().collect();
        println!("{}+{}+{}", s, s, s);
        fn show_one_branch(b: &&Branch) {
            let mut item_type_name = b.item_type_name();
            item_type_name.truncate(30);
            println!(
                "{:<30} | {:<30} | {:<30}",
                b.name(),
                item_type_name,
                b.interpretation()
            );
        }

        self.branches_r().iter().for_each(show_one_branch);
    }
}

impl Unmarshaler for Tree {
    fn unmarshal(&mut self, r: &mut RBuffer) -> anyhow::Result<()> {
        let hdr = r.read_header(self.class())?;
        ensure!(
            hdr.vers <= rvers::TREE,
            "rtree: invalid {} version={} > {}",
            self.class(),
            hdr.vers,
            rvers::TREE
        );

        self.rvers = hdr.vers;
        r.read_object(&mut self.named)?;
        r.read_object(&mut self.attline)?;
        r.read_object(&mut self.attfill)?;
        r.read_object(&mut self.attmarker)?;

        if hdr.vers <= 4 {
            bail!(
                "rtree: tree {} with version {} is not supported (too old)",
                self.name(),
                hdr.vers,
            )
        }

        if hdr.vers > 5 {
            self.entries = r.read_i64()?;
            self.tot_bytes = r.read_i64()?;
            self.zip_bytes = r.read_i64()?;
            self.saved_bytes = r.read_i64()?;
        } else {
            self.entries = r.read_f64()? as i64;
            self.tot_bytes = r.read_f64()? as i64;
            self.zip_bytes = r.read_f64()? as i64;
            self.saved_bytes = r.read_f64()? as i64;
        }

        if hdr.vers >= 18 {
            self.flushed_bytes = r.read_i64()?;
        }

        if hdr.vers >= 16 {
            self.weight = r.read_f64()?;
        }

        self.timer_interval = r.read_i32()?;
        self.scan_field = r.read_i32()?;
        self.update = r.read_i32()?;

        if hdr.vers >= 17 {
            self.default_entry_offset_len = r.read_i32()?;
        }

        let mut nclus = 0;

        if hdr.vers >= 19 {
            nclus = r.read_i32()?;
        }

        if hdr.vers > 5 {
            self.max_entries = r.read_i64()?;
        }

        if hdr.vers > 5 {
            self.max_entry_loop = r.read_i64()?;
            self.max_virtual_size = r.read_i64()?;
            self.auto_save = r.read_i64()?;
        } else {
            self.max_entry_loop = r.read_i32()? as i64;
            self.max_virtual_size = r.read_i32()? as i64;
            self.auto_save = r.read_i32()? as i64;
        }

        if hdr.vers >= 18 {
            self.auto_flush = r.read_i64()?;
        }

        if hdr.vers > 5 {
            self.estimate = r.read_i64()?;
        } else {
            self.estimate = r.read_i32()? as i64;
        }

        if hdr.vers >= 19 {
            self.clusters.ranges = vec![0; nclus as usize];
            self.clusters.sizes = vec![0; nclus as usize];
            let _ = r.read_i8();
            r.read_array_i64(&mut self.clusters.ranges)?;

            let _ = r.read_i8();
            r.read_array_i64(&mut self.clusters.sizes)?;
        }

        if hdr.vers >= 20 {
            r.read_object(&mut self.iobits)?;
        }

        {
            let mut branches = r.read_object_into::<ObjArray>()?;

            // for obj in branches.take_objs().into_iter() {
            //     trace!("convert branch");
            //     obj.downcast::<TBranch>().unwrap();
            // }

            self.branches = branches
                .take_objs()
                .into_iter()
                .map(|obj| obj.into())
                .collect();

            self.branches.iter_mut().for_each(|b| {
                b.set_top_level(Some(true));
                // b.set_item_type_name();
            });
        }

        {
            let mut _leaves = r.read_object_into::<ObjArray>()?;
        }

        if hdr.vers > 5 {
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }
        }

        {
            //tree.indexValues
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }
        }

        {
            //tree.index
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }
        }

        if hdr.vers > 5 {
            //tree.index
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }

            //tree.friends
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }

            //tree.userInfo
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }

            //tree.branchRef
            let v = r.read_object_any_into()?;
            if v.is_some() {
                todo!()
            }
        }

        Ok(())
        // todo!()
    }
}

factory_all_for_register_impl!(Tree, "TTree");

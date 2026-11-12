//! Exact native backend shapes; logical roles are never physical owner indices.
//!
//! Local-wireless and Quickplay waiting share one PracticeScene setup (13.0.5
//! `0x1790020`, reached identically from the general controller `0x2310570` and
//! the online controller `0x237e970`): one primary row/entry 0 and CPU rows/entries
//! 4/5/6, with no mode branch around that construction. They stay distinct variants
//! so a mode word is never inferred from a shape and every live-shape gate still runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout { Training, Ldn, Quickplay }
impl Layout {
    /// The two observed online waiting routes use the same native PracticeScene
    /// owner map 0/4/5/6. Hardware C108 logs #630-660 and #718-737 witness the
    /// 0x07010400 route; it was previously excluded even though stage loading
    /// admitted it. This only classifies a mode: scene() still requires active
    /// waiting melee0d, and native adapters still prove every live owner.
    /// Arenas and unobserved 0x070104 submodes remain excluded.
    pub const fn quickplay_family(mode:u32)->Option<u32> {
        let mode=mode & 0x7fffffff;
        if mode & 0xffffff00 == 0x07010100 {Some(0x07010100)}
        else if mode == 0x07010400 {Some(mode)} else {None}
    }
    pub const fn quickplay_waiting_mode(mode:u32)->bool {Self::quickplay_family(mode).is_some()}

    /// Value-only continuity across an observed match excursion. Never grants
    /// permission to call a practice API in a live match. The second online
    /// route uses melee10 (hardware #661-675 and #706-717), not melee0e.
    pub const fn continuation(initialized:bool,melee:u8,mode:u32)->Option<u32> {
        if !initialized {return None;}
        if mode==0x02010000 && matches!(melee,0x01|0x0d) {return Some(mode);}
        match Self::quickplay_family(mode) {
            Some(0x07010100) if matches!(melee,0x01|0x0d|0x0e)=>Some(0x07010100),
            Some(0x07010400) if matches!(melee,0x0d|0x0e|0x10)=>Some(0x07010400),
            _=>None,
        }
    }
    pub fn scene(initialized:bool,melee:u8,mode:u32,waiting_active:bool)->Option<Self> {
        match (initialized,melee,mode) {
            (true,0x20,0x04040000)=>Some(Self::Training),
            (true,0x0d,0x02010000) if waiting_active=>Some(Self::Ldn),
            (true,0x0d,mode) if waiting_active && Self::quickplay_waiting_mode(mode)=>Some(Self::Quickplay),
            _=>None,
        }
    }
    /// The shared waiting shape: visible 0/4, helpers 5/6, native selection indices.
    pub const fn waiting(self)->bool {!matches!(self,Self::Training)}
    pub const fn visible(self)->[u32;2] {match self {Self::Training=>[0,1],Self::Ldn|Self::Quickplay=>[0,4]}}
    pub const fn helpers(self)->[u32;2] {match self {Self::Training=>[2,3],Self::Ldn|Self::Quickplay=>[5,6]}}
    pub fn owners(self)->[usize;4] {let v=self.visible();let h=self.helpers();[v[0] as usize,v[1] as usize,h[0] as usize,h[1] as usize]}
    pub fn logical(self,physical:u32)->Option<usize> {self.visible().iter().position(|id|*id==physical)}
    pub fn helper(self,physical:usize)->Option<usize> {self.helpers().iter().position(|id|*id as usize==physical)}
    pub fn selector(self,physical:usize)->bool {physical!=0 && self.owners().contains(&physical)}
    pub fn shape(self,entries:&[usize;8])->bool {
        let ids=self.owners();
        entries.iter().enumerate().all(|(id,p)|(*p!=0)==ids.contains(&id))
            && ids.iter().enumerate().all(|(i,id)|ids[..i].iter().all(|other|entries[*id]!=entries[*other]))
    }
    /// `shape` plus exactly the registered entries for this layout. No
    /// other nonzero slot, moved identity or owner/extra alias is accepted.
    pub fn shape_with(self,entries:&[usize;8],extras:crate::engine::mii::capacity::Extras)->bool {
        if !extras.matches(self) {return false;}
        let Some(view)=extras.view(entries) else {return false;};
        self.shape(&view) && extras.slots().all(|(_,entry)|!view.contains(&entry))
    }
    /// Raw map facts shared by native waiting admission and Mii owner adapters.
    /// An identity registration never substitutes for this fresh physical check.
    pub fn waiting_map(self,primary:&[usize;8],backup:&[usize;8],count:u32,
        extras:crate::engine::mii::capacity::Extras)->bool {
        self.waiting() && count==self.count_with(extras) && *backup==[0;8]
            && self.shape_with(primary,extras)
            && primary.iter().all(|pointer|pointer.is_multiple_of(8))
    }
    /// Native entry count of a `shape_with` map.
    pub fn count_with(self,extras:crate::engine::mii::capacity::Extras)->u32 {(self.owners().len()+extras.count()) as u32}
    pub fn quantity(self,quantity:Option<u32>)->bool {match self {Self::Training=>matches!(quantity,Some(1..=3)),Self::Ldn|Self::Quickplay=>quantity.is_none()}}
}

#[cfg(test)] mod tests {
    lib_unit_tests! {
    use super::*;
     fn registered_waiting_map_preserves_all_native_guards() {
        use crate::engine::mii::capacity::Extras;
        for layout in [Layout::Ldn,Layout::Quickplay] {
            let base=[0x1000,0,0,0,0x4000,0x5000,0x6000,0];
            assert!(layout.waiting_map(&base,&[0;8],4,Extras::NONE));
            for entries in [[0x7000,0,0],[0x7000,0x3000,0],[0x7000,0x3000,0x2000]] {
                let extras=Extras::new_for(layout,entries).unwrap();
                let physical=extras.physical(&base);let count=4+extras.count() as u32;
                assert!(layout.waiting_map(&physical,&[0;8],count,extras));
                assert!(!layout.waiting_map(&physical,&[0;8],4,extras));
                assert!(!layout.waiting_map(&physical,&[0;8],count,Extras::NONE));
                for slot in 0..8 {
                    let mut backup=[0;8];backup[slot]=0x8000;
                    assert!(!layout.waiting_map(&physical,&backup,count,extras));
                    let mut changed=physical;changed[slot]=0x9000;
                    if !layout.owners().contains(&slot) {
                        assert!(!layout.waiting_map(&changed,&[0;8],count,extras));
                    }
                }
                let mut misaligned=physical;misaligned[4]+=1;
                assert!(!layout.waiting_map(&misaligned,&[0;8],count,extras));
            }
        }
    }

     fn exact_backend_shape_and_logical_cpu() {
        let map=[0x1000,0,0,0,0x2000,0x3000,0x4000,0];
        assert_eq!(Layout::scene(true,13,0x02010000,true),Some(Layout::Ldn));
        for (melee,mode) in [(1,0x02010000),(13,0x07010000),(32,0x02010000)] {
            assert_eq!(Layout::scene(true,melee,mode,true),None);
        }
        assert!(Layout::Ldn.shape(&map));assert!(!Layout::Training.shape(&map));
        assert_eq!(Layout::Ldn.logical(4),Some(1));assert_eq!(Layout::Ldn.logical(1),None);
        assert_eq!(Layout::Ldn.helper(5),Some(0));assert_eq!(Layout::Ldn.helper(6),Some(1));
        assert!(Layout::Ldn.quantity(None));assert!(!Layout::Ldn.quantity(Some(1)));
        assert!(Layout::Training.quantity(Some(1)));assert!(!Layout::Training.quantity(None));
        let mut alias=map;alias[6]=map[5];assert!(!Layout::Ldn.shape(&alias));
    }
     fn quickplay_waiting_shares_the_shape_but_keeps_its_own_mode_family() {
        for mode in [0x07010100,0x07010101,0x07010102,0x07010400,0x87010400] {
            assert_eq!(Layout::scene(true,0x0d,mode,true),Some(Layout::Quickplay));
            assert_eq!(Layout::scene(true,0x0d,mode,false),None);
            assert_eq!(Layout::scene(false,0x0d,mode,true),None);
        }
        // Arenas, unobserved submodes, both online match melees and other families stay out.
        for (melee,mode) in [(0x0d,0x07010300),(0x0d,0x07010401),(0x10,0x07010400),(0x0e,0x07010400),(0x0e,0x07010101),(0x01,0x07010101),(0x0d,0x08010100),(0x20,0x07010101)] {
            assert_eq!(Layout::scene(true,melee,mode,true),None);
        }
        assert_ne!(Layout::Quickplay,Layout::Ldn);
        assert!(Layout::Quickplay.waiting() && Layout::Ldn.waiting() && !Layout::Training.waiting());
        let map=[0x1000,0,0,0,0x2000,0x3000,0x4000,0];
        assert!(Layout::Quickplay.shape(&map));
        assert_eq!(Layout::Quickplay.visible(),Layout::Ldn.visible());
        assert_eq!(Layout::Quickplay.helpers(),Layout::Ldn.helpers());
        assert_eq!(Layout::Quickplay.owners(),Layout::Ldn.owners());
        assert_eq!(Layout::Quickplay.logical(4),Some(1));assert_eq!(Layout::Quickplay.logical(1),None);
        assert_eq!(Layout::Quickplay.helper(5),Some(0));assert_eq!(Layout::Quickplay.helper(6),Some(1));
        assert!(Layout::Quickplay.selector(5) && !Layout::Quickplay.selector(0) && !Layout::Quickplay.selector(2));
        assert!(Layout::Quickplay.quantity(None) && !Layout::Quickplay.quantity(Some(1)));
        let mut alias=map;alias[6]=map[5];assert!(!Layout::Quickplay.shape(&alias));
    }
     fn only_registered_training_extras_extend_the_shape() {
        use crate::engine::mii::capacity::Extras;
        let training=[0x10,0x20,0x30,0x40,0,0,0,0];
        assert!(Layout::Training.shape_with(&training,Extras::NONE));
        assert_eq!(Layout::Training.count_with(Extras::NONE),4);
        let mut created=training;created[7]=0x70;created[6]=0x60;
        let extras=Extras::new([0x70,0x60,0]).unwrap();
        assert!(Layout::Training.shape_with(&created,extras));
        assert_eq!(Layout::Training.count_with(extras),6);
        assert!(!Layout::Training.shape(&created),"plain shape stays exact");
        // Unregistered, moved, slot4 or aliased extras are refused.
        assert!(!Layout::Training.shape_with(&created,Extras::new([0x70,0,0]).unwrap()));
        let mut moved=created;moved[6]=0x61;assert!(!Layout::Training.shape_with(&moved,extras));
        let mut four=created;four[4]=0x44;assert!(!Layout::Training.shape_with(&four,extras));
        let mut alias=training;alias[7]=0x30;
        assert!(!Layout::Training.shape_with(&alias,Extras::new([0x30,0,0]).unwrap()));
        // Registered extras never extend a waiting layout.
        let mut waiting=[0x10,0,0,0,0x20,0x30,0x40,0];
        assert!(Layout::Ldn.shape_with(&waiting,Extras::NONE));
        waiting[7]=0x70;
        assert!(!Layout::Ldn.shape_with(&waiting,Extras::new([0x70,0,0]).unwrap()));
        assert!(!Layout::Quickplay.shape_with(&waiting,Extras::new([0x70,0,0]).unwrap()));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("registered_waiting_map_preserves_all_native_guards", registered_waiting_map_preserves_all_native_guards as fn()),
            ("exact_backend_shape_and_logical_cpu", exact_backend_shape_and_logical_cpu as fn()),
            ("quickplay_waiting_shares_the_shape_but_keeps_its_own_mode_family", quickplay_waiting_shares_the_shape_but_keeps_its_own_mode_family as fn()),
            ("only_registered_training_extras_extend_the_shape", only_registered_training_extras_extend_the_shape as fn()),
        ]);
    }
    }
}

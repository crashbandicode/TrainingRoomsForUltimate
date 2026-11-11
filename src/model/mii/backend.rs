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


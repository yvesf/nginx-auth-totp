use std::collections::VecDeque;

#[derive(Clone, Copy)]
enum TablePointer<R> where R: Clone + Copy {
    Link(usize),
    Route(R),
    RouteWithLink(usize, R),
    NotFound,
}

pub struct NoMatchingRoute;

pub type RouteMatch<'a, R> = Result<(R, &'a str), NoMatchingRoute>;

#[derive(Clone)]
pub struct RoutingTable<R> where R: Copy {
    tables: Vec<[TablePointer<R>; 128]>,
}

impl<R> RoutingTable<R> where R: Copy {
    pub fn new() -> RoutingTable<R> {
        let zero_table = [TablePointer::NotFound; 128];
        RoutingTable {
            tables: vec![zero_table],
        }
    }

    pub fn insert(&mut self, path: &str, route: R) {
        assert!(!path.is_empty());
        let mut index: usize = 0;
        let p: Vec<usize> = path.as_bytes().iter().map(|i| usize::from(*i)).collect();
        let mut path_queue = VecDeque::from(p);

        while path_queue.len() > 1 {
            let ch = path_queue.pop_front().unwrap() as usize % 128;
            match self.tables[index][ch] {
                TablePointer::NotFound => {
                    self.tables.push([TablePointer::NotFound; 128]);
                    let i_other_table = self.tables.len() - 1;
                    self.tables[index][ch] = TablePointer::Link(i_other_table);
                    path_queue.push_front(ch);
                }
                TablePointer::Link(i_other_table) => {
                    index = i_other_table as usize;
                }
                TablePointer::RouteWithLink(i_other_table, _) => {
                    index = i_other_table as usize;
                    path_queue.push_front(ch);
                }
                TablePointer::Route(route) => {
                    self.tables.push([TablePointer::NotFound; 128]);
                    let i_other_table = self.tables.len() - 1;
                    self.tables[index][ch] = TablePointer::RouteWithLink(i_other_table, route);
                    path_queue.push_front(ch);
                }
            }
        }
        // last character element in path
        let ch = path_queue.pop_front().unwrap() % 128;
        match self.tables[index][ch] {
            TablePointer::NotFound => {
                // slot is empty, just place the Route
                self.tables[index][ch] = TablePointer::Route(route)
            }
            TablePointer::Link(i) => {
                // slot is filled with link to longer path. Replace by RouteWithLink
                self.tables[index][ch] = TablePointer::RouteWithLink(i, route);
            }
            _ => {
                panic!("Not expected here, maybe duplicate")
            }
        }
    }

    pub fn match_path<'a>(&self, path: &'a str) -> RouteMatch<'a, R> {
        let mut table: &[TablePointer<R>; 128] = &self.tables[0];
        let path_bytes = path.as_bytes();
        let path_max_i = path_bytes.len() - 1;

        let mut i = 0;
        while i <= path_max_i {
            let ch = path_bytes[i] as usize % 128;
            match table[ch] {
                TablePointer::NotFound => {
                    return Err(NoMatchingRoute);
                }
                TablePointer::Link(i_other_table) => {
                    table = &self.tables[i_other_table];
                    i += 1;
                }
                TablePointer::RouteWithLink(i_other_table, route) => {
                    if i == path_max_i {
                        return Ok((route, Default::default()));
                    } else {
                        table = &self.tables[i_other_table];
                    }
                }
                TablePointer::Route(route) => {
                    return Ok((route, path.get(i + 1..=path_max_i).unwrap()));
                }
            }
        }
        Err(NoMatchingRoute)
    }
}

#[cfg(test)]
mod test1 {
    use super::*;
    use test::Bencher;

    #[derive(Clone, Copy)]
    enum Route {
        Login,
        Logout,
        Info,
        Check
    }

    #[bench]
    fn bench_1(b: &mut Bencher) {
        let mut r = RoutingTable::new();
        r.insert("/login", Route::Login);
        r.insert("/info", Route::Info);
        r.insert("/logout", Route::Logout);
        r.insert("/logout2", Route::Info);
        r.insert("/check", Route::Check);

        match r.tables[0][47] {
            TablePointer::Link(n) => assert!(n == 1),
            _ => panic!("Wrong"),
        }

        b.iter(|| {
            match r.match_path("/login") {
                RouteMatch::Match(Route::Login, rest) => assert_eq!(rest, ""),
                _ => panic!("Wrong")
            }
            match r.match_path("/logout") {
                RouteMatch::Match(Route::Logout, rest) => assert_eq!(rest, ""),
                _ => panic!("Wrong")
            }
            match r.match_path("/info") {
                RouteMatch::Match(Route::Info, rest) => assert_eq!(rest, ""),
                _ => panic!("Wrong")
            }
            match r.match_path("/logout2") {
                RouteMatch::Match(Route::Info, rest) => assert_eq!(rest, ""),
                _ => panic!("Wrong")
            }
            match r.match_path("/asdasdasd") {
                RouteMatch::None => (),
                _ => panic!("Wrong")
            }

            match r.match_path("/login/foo/bar") {
                RouteMatch::Match(Route::Login, rest) => assert_eq!(rest, "/foo/bar"),
                _ => panic!("Wrong")
            }
        })
    }
}
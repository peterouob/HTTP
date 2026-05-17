macro_rules! next {
    ($bytes: ident) => {
        match $bytes.next() {
            Some(b) => b,
            None => return Ok(Status::Partial)
        }
    };
}

macro_rules! expect {
    ($bytes: ident.next() == $pat: pat_param => $ret:expr) => {
        expect!(next!($bytes) => $pat => $ret)
    };

    ($e: expr => $pat: pat_param => $ret:expr) => {
        match $e {
            v@$pat => v,
            _ => return $ret,
        }
    };
}

macro_rules! complete {
    ($e:expr) => {
        match $e? {
            Status::Complete(v) => v,
            Status::Partial => return Ok(Status::Partial),
        }
    };
}



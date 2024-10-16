use std::env;

pub struct CollConfig {
   
    pub coll_h_lmtorder: String,
    pub coll_h_mktorder: String,
    pub coll_h_sorder: String,
    pub coll_h_slorder: String,
    pub coll_h_modforder: String,
    pub coll_h_dltorder: String,
    pub coll_p_order: String,
    pub coll_p_sorder: String,
    pub coll_p_slorder: String,
    pub coll_h_dltd_order: String,
    pub coll_h_dltd_sorder: String,
    pub coll_h_dltd_slorder: String,
    pub coll_h_modfd_order: String,
    pub coll_h_modfd_sorder: String,
    pub coll_h_modfd_slorder: String,
    pub coll_h_exctd_sorder: String,
    pub coll_h_exctd_slorder: String,
    pub coll_t_message: String,
    pub coll_h_match: String,
    pub coll_h_pnl_no_cmmss: String,
    pub coll_a_position: String,
    pub coll_h_clsdpos: String,
    pub coll_h_dltd_iceberg: String,
    pub coll_h_exctd_iceberg: String,
    pub coll_h_iceberg: String,
    pub coll_h_modfd_iceberg: String,
    pub coll_p_iceberg: String,
    pub coll_h_modficeberg: String,
    pub coll_h_dlticeberg: String,
    pub coll_h_cmmss_paid: String,
    pub coll_trdr_bal: String,
    pub coll_h_pnl_w_cmmss: String,
    pub commission: i32,

    pub coll_h_last: String,
    pub coll_h_mbpevent: String,
    pub coll_h_interestevent: String,
    pub coll_h_bbo: String,
    pub coll_h_tns: String,
    pub coll_h_volume: String,
    pub coll_fullob: String,
    pub coll_fullinterest: String,

}

impl CollConfig {
    pub fn new() -> Self {
        Self {
          
            coll_h_lmtorder: env::var("COLL_H_LMTORDER").unwrap_or_else(|_| {
                eprintln!("COLL_H_LMTORDER not set, using default value.");
                std::process::exit(1);
            }),
            coll_h_mktorder: env::var("COLL_H_MKTORDER").unwrap_or_else(|_| {
                eprintln!("COLL_H_MKTORDER not set, using default value.");
                std::process::exit(1);
            }),
            coll_h_sorder: env::var("COLL_H_SORDER").unwrap_or_else(|_| {
                eprintln!("COLL_H_SORDER not set, using default value.");
                std::process::exit(1);
            }),
            coll_h_slorder:env::var("COLL_H_SLORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_SLORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_modforder:env::var("COLL_H_MODFORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_MODFORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_dltorder:env::var("COLL_H_DLTORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_DLTORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_p_order:env::var("COLL_P_ORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_P_ORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_p_sorder:env::var("COLL_P_SORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_P_SORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_p_slorder:env::var("COLL_P_SLORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_P_SLORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_dltd_order:env::var("COLL_H_DLTD_ORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_DLTD_ORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_dltd_sorder:env::var("COLL_H_DLTD_SORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_DLTD_SORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_dltd_slorder:env::var("COLL_H_DLTD_SLORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_DLTD_SLORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_modfd_order:env::var("COLL_H_MODFD_ORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_MODFD_ORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_modfd_sorder:env::var("COLL_H_MODFD_SORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_MODFD_SORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_modfd_slorder:env::var("COLL_H_MODFD_SLORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_MODFD_SLORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_exctd_sorder:env::var("COLL_H_EXCTD_SORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_EXCTD_SORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_exctd_slorder:env::var("COLL_H_EXCTD_SLORDER")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_EXCTD_SLORDER not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_t_message:env::var("COLL_T_MESSAGE")
            .unwrap_or_else(|_| {
                eprintln!("COLL_T_MESSAGE not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_match:env::var("COLL_H_MATCH")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_TRADE not set, using default collection.");
                std::process::exit(1); 
            }),
    
        coll_h_pnl_no_cmmss:env::var("COLL_H_PNL_NO_CMMSS")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_PNL_NO_CMMSS not set, using default collection.");
                std::process::exit(1); 
            }),
            coll_a_position:env::var("COLL_A_POSITION")
            .unwrap_or_else(|_| {
                eprintln!("COLL_A_POSITION not set, using default collection.");
                std::process::exit(1); 
            }),
            coll_h_clsdpos:env::var("COLL_H_CLSDPOS")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_CLSDPOS not set, using default collection.");
                std::process::exit(1); 
            }),
            coll_h_dltd_iceberg:env::var("COLL_H_DLTD_ICEBERG")
        .unwrap_or_else(|_| {
            eprintln!("COLL_H_DLTD_ICEBERG not set, using default collection.");
            std::process::exit(1);
        }),
    
        coll_h_exctd_iceberg:env::var("COLL_H_EXCTD_ICEBERG")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_EXCTD_ICEBERG not set, using default collection.");
                std::process::exit(1);
            }),
    
        coll_h_iceberg:env::var("COLL_H_ICEBERG")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_ICEBERG not set, using default collection.");
                std::process::exit(1);
            }),
    
        coll_h_modfd_iceberg:env::var("COLL_H_MODFD_ICEBERG")
            .unwrap_or_else(|_| {
                eprintln!("COLL_H_MODFD_ICEBERG not set, using default collection.");
                std::process::exit(1);
            }),
    
        coll_p_iceberg:env::var("COLL_P_ICEBERG")
            .unwrap_or_else(|_| {
                eprintln!("COLL_P_ICEBERG not set, using default collection.");
                std::process::exit(1);
            }),
    
            coll_h_modficeberg:env::var("COLL_H_MODFICEBERG")
        .unwrap_or_else(|_| {
            eprintln!("COLL_H_MODFICEBERG not set, using default collection.");
            std::process::exit(1);
        }),
    
    coll_h_dlticeberg:env::var("COLL_H_DLTICEBERG")
        .unwrap_or_else(|_| {
            eprintln!("COLL_H_DLTICEBERG not set, using default collection.");
            std::process::exit(1);
        }),
    
        coll_h_cmmss_paid:env::var("COLL_H_CMMSS_PAID")
        .unwrap_or_else(|_| {
            eprintln!("COLL_H_CMMSS_PAID not set, using default collection.");
            std::process::exit(1);
        }),
    
    coll_trdr_bal:env::var("COLL_TRDR_BAL")
        .unwrap_or_else(|_| {
            eprintln!("COLL_TRDR_BAL not set, using default collection.");
            std::process::exit(1);
        }),
    
    coll_h_pnl_w_cmmss:env::var("COLL_H_PNL_W_CMMSS")
        .unwrap_or_else(|_| {
            eprintln!("COLL_H_PNL_W_CMMSS not set, using default collection.");
            std::process::exit(1);
        }),
    
    commission:env::var("COMMISSION")
        .unwrap_or_else(|_| {
            eprintln!("COMMISSION not set, using default value.");
            std::process::exit(1);
        })
        .parse()
        .unwrap_or_else(|_| {
            eprintln!("COMMISSION is not a valid integer.");
            std::process::exit(1);
        }),

        /////////////////////////////////////////
        coll_h_last: env::var("COLL_H_LAST").unwrap_or_else(|_| {
            eprintln!("COLL_H_LAST not set, using default value.");
            std::process::exit(1);
        }),
        coll_h_mbpevent: env::var("COLL_H_MBP_EVENT").unwrap_or_else(|_| {
            eprintln!("COLL_H_MBP_EVENT not set, using default value.");
            std::process::exit(1);
        }),
        coll_h_interestevent: env::var("COLL_H_INTEREST_EVENT").unwrap_or_else(|_| {
            eprintln!("COLL_H_INTEREST_EVENT not set, using default value.");
            std::process::exit(1);
        }),
        coll_h_bbo: env::var("COLL_H_BBO").unwrap_or_else(|_| {
            eprintln!("COLL_H_BBO not set, using default value.");
            std::process::exit(1);
        }),
        coll_h_tns: env::var("COLL_H_TNS").unwrap_or_else(|_| {
            eprintln!("COLL_H_TNS not set, using default value.");
            std::process::exit(1);
        }),
        coll_h_volume: env::var("COLL_H_VOLUME").unwrap_or_else(|_| {
            eprintln!("COLL_H_VOLUME not set, using default value.");
            std::process::exit(1);
        }),
        coll_fullob: env::var("COLL_FULL_OB").unwrap_or_else(|_| {
            eprintln!("COLL_FULL_OB not set, using default value.");
            std::process::exit(1);
        }),
        coll_fullinterest: env::var("COLL_FULL_INTEREST").unwrap_or_else(|_| {
            eprintln!("COLL_FULL_INTEREST not set, using default value.");
            std::process::exit(1);
        }),
        }
    }
}

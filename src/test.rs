// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn verify_keep_app_active() {
//         let happs_to_keep = vec!["elemental-chat:2".to_string(), "hha:1".to_string()];
//         let app_1 = "elemental-chat:1";
//         let app_2 = "elemental-chat:2";
//         let app_3 = "uhCkkcF0X1dpwHFeIPI6-7rzM6ma9IgyiqD-othxgENSkL1So1Slt::servicelogger";
//         let app_4 = "other-app";

//         assert_eq!(_keep_app_active(app_1, happs_to_keep.clone()), false);
//         assert_eq!(_keep_app_active(app_2, happs_to_keep.clone()), true); // because it is in config
//         assert_eq!(_keep_app_active(app_3, happs_to_keep.clone()), true); // because it is hosted
//         assert_eq!(_keep_app_active(app_4, happs_to_keep.clone()), false);
//     }
// }

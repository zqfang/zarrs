use rayon::iter::{IntoParallelIterator, ParallelIterator};
use zarrs::{
    array::storage_transformer::{StorageTransformerExtension, UsageLogStorageTransformer},
    storage::ReadableWritableListableStorage,
};

fn array_write_read() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use zarrs::{
        array::{DataType, FillValue, ZARR_NAN_F32},
        array_subset::ArraySubset,
        node::Node,
        storage::store,
    };

    // Create a store
    // let path = tempfile::TempDir::new()?;
    // let mut store: ReadableWritableListableStorage = Arc::new(store::FilesystemStore::new(path.path())?);
    // let mut store: ReadableWritableListableStorage = Arc::new(store::FilesystemStore::new(
    //     "tests/data/array_write_read.zarr",
    // )?);
    let mut store: ReadableWritableListableStorage = Arc::new(store::MemoryStore::new());
    if let Some(arg1) = std::env::args().collect::<Vec<_>>().get(1) {
        if arg1 == "--usage-log" {
            let log_writer = Arc::new(std::sync::Mutex::new(
                // std::io::BufWriter::new(
                std::io::stdout(),
                //    )
            ));
            let usage_log = Arc::new(UsageLogStorageTransformer::new(log_writer, || {
                chrono::Utc::now().format("[%T%.3f] ").to_string()
            }));
            store = usage_log
                .clone()
                .create_readable_writable_listable_transformer(store);
        }
    }

    // Create a group
    let group_path = "/group";
    let mut group = zarrs::group::GroupBuilder::new().build(store.clone(), group_path)?;

    // Update group metadata
    group
        .attributes_mut()
        .insert("foo".into(), serde_json::Value::String("bar".into()));

    // Write group metadata to store
    group.store_metadata()?;

    println!(
        "The group metadata is:\n{}\n",
        serde_json::to_string_pretty(&group.metadata()).unwrap()
    );

    // Create an array
    let array_path = "/group/array";
    let array = zarrs::array::ArrayBuilder::new(
        vec![8, 8], // array shape
        DataType::Float32,
        vec![4, 4].try_into()?, // regular chunk shape
        FillValue::from(ZARR_NAN_F32),
    )
    // .bytes_to_bytes_codecs(vec![]) // uncompressed
    .dimension_names(["y", "x"].into())
    // .storage_transformers(vec![].into())
    .build(store.clone(), array_path)?;

    // Write array metadata to store
    array.store_metadata()?;

    println!(
        "The array metadata is:\n{}\n",
        serde_json::to_string_pretty(&array.metadata()).unwrap()
    );

    // Write some chunks
    (0..2).into_par_iter().try_for_each(|i| {
        let chunk_indices: Vec<u64> = vec![0, i];
        let chunk_subset = array
            .chunk_grid()
            .subset(&chunk_indices, array.shape())?
            .ok_or_else(|| {
                zarrs::array::ArrayError::InvalidChunkGridIndicesError(chunk_indices.to_vec())
            })?;
        array.store_chunk_elements(
            &chunk_indices,
            &vec![i as f32 * 0.1; chunk_subset.num_elements() as usize],
        )
    })?;

    let subset_all = ArraySubset::new_with_shape(array.shape().to_vec());
    let data_all = array.retrieve_array_subset_ndarray::<f32>(&subset_all)?;
    println!("store_chunk [0, 0] and [0, 1]:\n{data_all:+4.1}\n");

    // Store multiple chunks
    array.store_chunks_elements::<f32>(
        &ArraySubset::new_with_ranges(&[1..2, 0..2]),
        &[
            //
            1.0, 1.0, 1.0, 1.0, 1.1, 1.1, 1.1, 1.1, 1.0, 1.0, 1.0, 1.0, 1.1, 1.1, 1.1, 1.1,
            //
            1.0, 1.0, 1.0, 1.0, 1.1, 1.1, 1.1, 1.1, 1.0, 1.0, 1.0, 1.0, 1.1, 1.1, 1.1, 1.1,
        ],
    )?;
    let data_all = array.retrieve_array_subset_ndarray::<f32>(&subset_all)?;
    println!("store_chunks [1..2, 0..2]:\n{data_all:+4.1}\n");

    // Write a subset spanning multiple chunks, including updating chunks already written
    array.store_array_subset_elements::<f32>(
        &ArraySubset::new_with_ranges(&[3..6, 3..6]),
        &[-3.3, -3.4, -3.5, -4.3, -4.4, -4.5, -5.3, -5.4, -5.5],
    )?;
    let data_all = array.retrieve_array_subset_ndarray::<f32>(&subset_all)?;
    println!("store_array_subset [3..6, 3..6]:\n{data_all:+4.1}\n");

    // Store array subset
    array.store_array_subset_elements::<f32>(
        &ArraySubset::new_with_ranges(&[0..8, 6..7]),
        &[-0.6, -1.6, -2.6, -3.6, -4.6, -5.6, -6.6, -7.6],
    )?;
    let data_all = array.retrieve_array_subset_ndarray::<f32>(&subset_all)?;
    println!("store_array_subset [0..8, 6..7]:\n{data_all:+4.1}\n");

    // Store chunk subset
    array.store_chunk_subset_elements::<f32>(
        // chunk indices
        &[1, 1],
        // subset within chunk
        &ArraySubset::new_with_ranges(&[3..4, 0..4]),
        &[-7.4, -7.5, -7.6, -7.7],
    )?;
    let data_all = array.retrieve_array_subset_ndarray::<f32>(&subset_all)?;
    println!("store_chunk_subset [3..4, 0..4] of chunk [1, 1]:\n{data_all:+4.1}\n");

    // Erase a chunk
    array.erase_chunk(&[0, 0])?;
    let data_all = array.retrieve_array_subset_ndarray::<f32>(&subset_all)?;
    println!("erase_chunk [0, 0]:\n{data_all:+4.1}\n");

    // Read a chunk
    let chunk_indices = vec![0, 1];
    let data_chunk = array.retrieve_chunk_ndarray::<f32>(&chunk_indices)?;
    println!("retrieve_chunk [0, 1]:\n{data_chunk:+4.1}\n");

    // Read chunks
    let chunks = ArraySubset::new_with_ranges(&[0..2, 1..2]);
    let data_chunks = array.retrieve_chunks_ndarray::<f32>(&chunks)?;
    println!("retrieve_chunks [0..2, 1..2]:\n{data_chunks:+4.1}\n");

    // Retrieve an array subset
    let subset = ArraySubset::new_with_ranges(&[2..6, 3..5]); // the center 4x2 region
    let data_subset = array.retrieve_array_subset_ndarray::<f32>(&subset)?;
    println!("retrieve_array_subset [2..6, 3..5]:\n{data_subset:+4.1}\n");

    // Show the hierarchy
    let node = Node::open(&store, "/").unwrap();
    let tree = node.hierarchy_tree();
    println!("hierarchy_tree:\n{}", tree);

    Ok(())
}

fn main() {
    if let Err(err) = array_write_read() {
        println!("{:?}", err);
    }
}

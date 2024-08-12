use pyo3::exceptions::{PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use zarrs::array::{Array as RustArray};
use zarrs::array_subset::ArraySubset;
use zarrs::storage::ReadableStorageTraits;
use pyo3::types::{PyInt, PyList, PySlice, PyTuple};
use std::ops::Range;
use dlpark::prelude::*;
use std::ffi::c_void;


#[pyclass]
pub struct ZarrsPythonArray {
    pub arr: RustArray<dyn ReadableStorageTraits + 'static>
}

impl ZarrsPythonArray {

    fn maybe_convert_u64(&self, ind: i32, axis: usize) -> PyResult<u64> {
        let mut ind_u64: u64 = ind as u64;
        if ind < 0 {
            if self.arr.shape()[axis] as i32 + ind < 0 {
                return Err(PyIndexError::new_err(format!("{0} out of bounds", ind)))
            }
            ind_u64 = u64::try_from(ind).map_err(|_| PyIndexError::new_err("Failed to extract start"))?;
        }
        return Ok(ind_u64);
    }

    fn bound_slice(&self, slice: &Bound<PySlice>, axis: usize) -> PyResult<Range<u64>> {
        let start: i32 = slice.getattr("start")?.extract().map_or(0, |x| x);
        let stop: i32 = slice.getattr("stop")?.extract().map_or(self.arr.shape()[axis] as i32, |x| x);
        let start_u64 = self.maybe_convert_u64(start, 0)?;
        let stop_u64 = self.maybe_convert_u64(stop, 0)?;
        // let _step: u64 = slice.getattr("step")?.extract().map_or(1, |x| x); // there is no way to use step it seems with zarrs?
        let selection = start_u64..stop_u64;
        return Ok(selection)
    }

    pub fn fill_from_slices(&self, slices: Vec<Range<u64>>) -> PyResult<Vec<Range<u64>>> {
        Ok(self.arr.shape().iter().enumerate().map(|(index, &value)| { if index < slices.len() { slices[index].clone() } else { 0..value } }).collect())
    }
}

#[pymethods]
impl ZarrsPythonArray {

    pub fn retrieve_chunk_subset(&self, chunk_coords_and_selections: &Bound<'_, PyList>) -> PyResult<ManagerCtx<PyZarrArr>> {
        if let Ok(chunk_coords_and_selection_list) = chunk_coords_and_selections.downcast::<PyList>() {
            let coords_extracted: Vec<Vec<u64>> = vec![vec![0]; chunk_coords_and_selection_list.len()];
            let selections_extracted: Vec<ArraySubset> = vec![ArraySubset::new_empty(1); chunk_coords_and_selection_list.len()];
            chunk_coords_and_selection_list.into_iter().enumerate().map(|(index, chunk_coord_and_selection)| {
                if let Ok(chunk_coord_and_selection_tuple) = chunk_coord_and_selection.downcast::<PyTuple>() {
                    let coord = chunk_coord_and_selection_tuple.get_item(0)?;
                    let coord_extracted: Vec<u64>;
                    if let Ok(coord_downcast) = coord.downcast::<PyTuple>() {
                        coord_extracted = coord_downcast.extract()?;
                        coords_extracted[index] = coord_extracted;
                    } else {
                        return Err(PyValueError::new_err(format!("Cannot take {0}, must be int or slice", coord.to_string())));
                    }
                    let selection = chunk_coord_and_selection_tuple.get_item(1)?;
                    let selection_extracted: ArraySubset;
                    if let Ok(slice) = selection.downcast::<PySlice>() {
                        selections_extracted[index] = ArraySubset::new_with_ranges(&self.fill_from_slices(vec![self.bound_slice(slice, 0)?])?);
                    } else if let Ok(tuple) = selection.downcast::<PyTuple>(){
                        let ranges: Vec<Range<u64>> = tuple.into_iter().enumerate().map(|(index, val)| {
                            if let Ok(int) = val.downcast::<PyInt>() {
                                let end = self.maybe_convert_u64(int.extract()?, index)?;
                                Ok(end..(end + 1))
                            } else if let Ok(slice) = val.downcast::<PySlice>() {
                                Ok(self.bound_slice(slice, index)?)
                            } else {
                                return Err(PyValueError::new_err(format!("Cannot take {0}, must be int or slice", val.to_string())));
                            }
                        }).collect::<Result<Vec<Range<u64>>, _>>()?;
                        selections_extracted[index] = ArraySubset::new_with_ranges(&self.fill_from_slices(ranges)?);
                    } else {
                        return Err(PyTypeError::new_err(format!("Unsupported type: {0}", selection)));
                    }
                }
                return Err(PyTypeError::new_err(format!("Unsupported type: {0}", chunk_coord_and_selection)));
            });
        } else {
            return Err(PyTypeError::new_err(format!("Unsupported type: {0}", chunk_coords)));
        }
        let arr = self.arr.retrieve_chunk_subset(&coords, &selection).map_err(|x| PyErr::new::<PyTypeError, _>(x.to_string()))?;
        let shape = selection.shape().iter().map(|&x| x as i64).collect::<Vec<i64>>();
        Ok(ManagerCtx::new(PyZarrArr{ shape, arr }))
    }
}


pub struct PyZarrArr {
    arr: Vec<u8>,
    shape: Vec<i64>,
}

impl ToTensor for PyZarrArr { 
    fn data_ptr(&self) -> *mut std::ffi::c_void {
        self.arr.as_ptr() as *const c_void as *mut c_void
    }
    fn shape_and_strides(&self) -> ShapeAndStrides {
        ShapeAndStrides::new_contiguous_with_strides(
            self.shape.iter()
        )
    }

    fn byte_offset(&self) -> u64 {
        0
    }


    fn device(&self) -> Device {
        Device::CPU
    }

    fn dtype(&self) -> DataType {
        DataType::U8
    }
 }
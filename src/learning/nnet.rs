//! Neural Network module
//!
//! Contains implementation of simple feed forward neural network.
//!
//! # Usage
//!
//! ```
//! use rusty_machine::learning::nnet::{NeuralNet, BCECriterion};
//! use rusty_machine::learning::toolkit::regularization::Regularization;
//! use rusty_machine::learning::toolkit::activ_fn::Sigmoid;
//! use rusty_machine::learning::optim::grad_desc::StochasticGD;
//! use rusty_machine::linalg::Matrix;
//! use rusty_machine::learning::SupModel;
//!
//! let inputs = Matrix::new(5,3, vec![1.,1.,1.,2.,2.,2.,3.,3.,3.,
//!                                 4.,4.,4.,5.,5.,5.,]);
//! let targets = Matrix::new(5,3, vec![1.,0.,0.,0.,1.,0.,0.,0.,1.,
//!                                     0.,0.,1.,0.,0.,1.]);
//!
//! // Set the layer sizes - from input to output
//! let layers = &[3,5,11,7,3];
//!
//! // Choose the BCE criterion with L2 regularization (`lambda=0.1`).
//! let criterion = BCECriterion::new(Regularization::L2(0.1));
//!
//! // We will just use the default stochastic gradient descent.
//! let mut model = NeuralNet::mlp(layers, criterion, StochasticGD::default(), Sigmoid);
//!
//! // Train the model!
//! model.train(&inputs, &targets);
//!
//! let test_inputs = Matrix::new(2,3, vec![1.5,1.5,1.5,5.1,5.1,5.1]);
//!
//! // And predict new output from the test inputs
//! model.predict(&test_inputs);
//! ```
//!
//! The neural networks are specified via a criterion - similar to
//! [Torch](https://github.com/torch/nn/blob/master/doc/criterion.md).
//! The criterions combine an activation function and a cost function.
//!
//! You can define your own criterion by implementing the `Criterion`
//! trait with a concrete `ActivationFunc` and `CostFunc`.

use linalg::{Matrix, MatrixSlice};
use linalg::BaseSlice;

use learning::SupModel;
use learning::toolkit::activ_fn;
use learning::toolkit::activ_fn::ActivationFunc;
use learning::toolkit::cost_fn;
use learning::toolkit::cost_fn::CostFunc;
use learning::toolkit::regularization::Regularization;
use learning::toolkit::net_layer;
use learning::toolkit::net_layer::NetLayer;
use learning::optim::{Optimizable, OptimAlgorithm};
use learning::optim::grad_desc::StochasticGD;

use rand::thread_rng;
use rand::distributions::{Sample, range};

use std::fmt::Debug;

/// Neural Network Model
///
/// The Neural Network struct specifies a Criterion and
/// a gradient descent algorithm.
#[derive(Debug)]
pub struct NeuralNet<T, A>
    where T: Criterion,
          A: OptimAlgorithm<BaseNeuralNet<T>>
{
    base: BaseNeuralNet<T>,
    alg: A,
}

/// Supervised learning for the Neural Network.
///
/// The model is trained using back propagation.
impl<T, A> SupModel<Matrix<f64>, Matrix<f64>> for NeuralNet<T, A>
    where T: Criterion,
          A: OptimAlgorithm<BaseNeuralNet<T>>
{
    /// Predict neural network output using forward propagation.
    fn predict(&self, inputs: &Matrix<f64>) -> Matrix<f64> {
        self.base.forward_prop(inputs)
    }

    /// Train the model using gradient optimization and back propagation.
    fn train(&mut self, inputs: &Matrix<f64>, targets: &Matrix<f64>) {
        let optimal_w = self.alg.optimize(&self.base, &self.base.weights, inputs, targets);
        self.base.weights = optimal_w;
    }
}

impl NeuralNet<BCECriterion, StochasticGD> {
    /// Creates a neural network with the specified layer sizes.
    ///
    /// The layer sizes slice should include the input, hidden layers, and output layer sizes.
    /// The type of activation function must be specified.
    ///
    /// Uses the default settings (stochastic gradient descent and sigmoid activation function).
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::learning::nnet::NeuralNet;
    ///
    /// // Create a neural net with 4 layers, 3 neurons in each.
    /// let layers = &[3; 4];
    /// let mut net = NeuralNet::default(layers);
    /// ```
    pub fn default(layer_sizes: &[usize]) -> NeuralNet<BCECriterion, StochasticGD> {
        NeuralNet {
            base: BaseNeuralNet::default(layer_sizes, activ_fn::Sigmoid),
            alg: StochasticGD::default(),
        }
    }
}

impl<T, A> NeuralNet<T, A>
    where T: Criterion,
          A: OptimAlgorithm<BaseNeuralNet<T>>
{
    /// Create a new neural network with no layers
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::learning::nnet::BCECriterion;
    /// use rusty_machine::learning::nnet::NeuralNet;
    /// use rusty_machine::learning::optim::grad_desc::StochasticGD;
    ///
    /// // Create a an empty neural net
    /// let mut net = NeuralNet::new(BCECriterion::default(), StochasticGD::default());
    /// ```
    pub fn new(criterion: T, alg: A) -> NeuralNet<T, A> {
        NeuralNet {
            base: BaseNeuralNet::new(criterion),
            alg: alg,
        }
    }

    /// Create a multilayer perceptron with the specified layer sizes.
    ///
    /// The layer sizes slice should include the input, hidden layers, and output layer sizes.
    /// The type of activation function must be specified.
    ///
    /// Currently defaults to simple batch Gradient Descent for optimization.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::learning::nnet::BCECriterion;
    /// use rusty_machine::learning::nnet::NeuralNet;
    /// use rusty_machine::learning::toolkit::activ_fn::Sigmoid;
    /// use rusty_machine::learning::optim::grad_desc::StochasticGD;
    ///
    /// // Create a neural net with 4 layers, 3 neurons in each.
    /// let layers = &[3; 4];
    /// let mut net = NeuralNet::mlp(layers, BCECriterion::default(), StochasticGD::default(), Sigmoid);
    /// ```
    pub fn mlp<U>(layer_sizes: &[usize], criterion: T, alg: A, activ_fn: U) -> NeuralNet<T, A> 
        where U: ActivationFunc + 'static {
        NeuralNet {
            base: BaseNeuralNet::mlp(layer_sizes, criterion, activ_fn),
            alg: alg,
        }
    }

    /// Adds the specified layer to the end of the network
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::linalg::BaseSlice;
    /// use rusty_machine::learning::nnet::BCECriterion;
    /// use rusty_machine::learning::nnet::NeuralNet;
    /// use rusty_machine::learning::optim::grad_desc::StochasticGD;
    /// use rusty_machine::learning::toolkit::net_layer::Linear;
    ///
    /// // Create a new neural net 
    /// let mut net = NeuralNet::new(BCECriterion::default(), StochasticGD::default());
    ///
    /// // Give net an input layer of size 3, hidden layer of size 4, and output layer of size 5
    /// net.add_layer(Box::new(Linear::with_bias(3, 4)));
    /// net.add_layer(Box::new(Linear::with_bias(4, 5)));
    /// ```
    pub fn add_layer<'a>(&'a mut self, layer: Box<NetLayer>) -> &'a mut NeuralNet<T, A> {
        self.base.add_layer(layer);
        self
    }

    /// Gets matrix of weights between specified layer and forward layer.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::linalg::BaseSlice;
    /// use rusty_machine::learning::nnet::NeuralNet;
    ///
    /// // Create a neural net with 4 layers, 3 neurons in each.
    /// let layers = &[3; 4];
    /// let mut net = NeuralNet::default(layers);
    ///
    /// let w = &net.get_net_weights(2);
    ///
    /// // We add a bias term to the weight matrix
    /// assert_eq!(w.rows(), 4);
    /// assert_eq!(w.cols(), 3);
    /// ```
    pub fn get_net_weights(&self, idx: usize) -> MatrixSlice<f64> {
        self.base.get_layer_weights(&self.base.weights[..], idx)
    }
}

/// Base Neural Network struct
///
/// This struct cannot be instantianated and is used internally only.
#[derive(Debug)]
pub struct BaseNeuralNet<T: Criterion> {
    layers: Vec<Box<NetLayer>>,
    weights: Vec<f64>,
    criterion: T,
}


impl BaseNeuralNet<BCECriterion> {
    /// Creates a base neural network with the specified layer sizes.
    fn default<U>(layer_sizes: &[usize], activ_fn: U) -> BaseNeuralNet<BCECriterion>
        where U: ActivationFunc + Debug + 'static {
        BaseNeuralNet::mlp(layer_sizes, BCECriterion::default(), activ_fn)
    }
}


impl<T: Criterion> BaseNeuralNet<T> {
    /// Create a base neural network with no layers
    fn new(criterion: T) -> BaseNeuralNet<T> {
        BaseNeuralNet {
            layers: Vec::new(),
            weights: Vec::new(),
            criterion: criterion
        }
    }

    /// Create a multilayer perceptron with the specified layer sizes.
    fn mlp<'a, U>(layer_sizes: &[usize], criterion: T, activ_fn: U) -> BaseNeuralNet<T> 
        where U: ActivationFunc + 'static {
        let mut mlp = BaseNeuralNet::new(criterion);
        for shape in layer_sizes.windows(2) {
            mlp.add_layer(Box::new(net_layer::Linear::with_bias(shape[0], shape[1])));
            mlp.add_layer(Box::new(activ_fn.clone()));
        }
        mlp
    }

    /// Adds the specified layer to the end of the network
    fn add_layer<'a>(&'a mut self, layer: Box<NetLayer>) -> &'a mut BaseNeuralNet<T> {
        self.weights.append(&mut layer.default_params());
        self.layers.push(layer);
        self
    }

    /// Creates initial weights for all neurons in the network.
    fn create_weights(layer_sizes: &[usize]) -> Vec<f64> {
        let mut between = range::Range::new(0f64, 1f64);
        let mut rng = thread_rng();
        layer_sizes
            .windows(2)
            .flat_map(|w| {
                let l_in = w[0] + 1;
                let l_out = w[1];
                let eps_init = (6f64 / (l_in + l_out) as f64).sqrt();
                (0..l_in * l_out)
                    .map(|_i| (between.sample(&mut rng) * 2f64 * eps_init) - eps_init)
                    .collect::<Vec<_>>()
            }).collect()
    }

    /// Gets matrix of weights for the specified layer for the weights.
    fn get_layer_weights(&self, weights: &[f64], idx: usize) -> MatrixSlice<f64> {
        debug_assert!(idx < self.layers.len());

        // Check that the weights are the right size.
        let mut full_size = 0usize;
        for l in &self.layers {
            full_size += l.num_params();
        }

        debug_assert_eq!(full_size, weights.len());

        let mut start = 0usize;
        for l in &self.layers[..idx] {
            start += l.num_params();
        } 

        let shape = self.layers[idx].param_shape();
        unsafe {
            MatrixSlice::from_raw_parts(weights.as_ptr().offset(start as isize),
                                        shape.0,
                                        shape.1,
                                        shape.1)
        }
    }

    /// Gets matrix of weights between specified layer and forward layer
    /// for the base model.
    fn get_net_weights(&self, idx: usize) -> MatrixSlice<f64> {
        self.get_layer_weights(&self.weights[..], idx)
    }

    /// Gets the weights for a layer excluding the bias weights.
    fn get_non_bias_weights(&self, weights: &[f64], idx: usize) -> MatrixSlice<f64> {
        let layer_weights = self.get_layer_weights(weights, idx);
        layer_weights.reslice([1, 0], layer_weights.rows() - 1, layer_weights.cols())
    }

    /// Compute the gradient using the back propagation algorithm.
    fn compute_grad(&self,
                    weights: &[f64],
                    inputs: &Matrix<f64>,
                    targets: &Matrix<f64>)
                    -> (f64, Vec<f64>) {
        let mut gradients = Vec::with_capacity(weights.len());
        unsafe {
            gradients.set_len(weights.len());
        }
        //activations[0] is input and activations[i+1] is output of layer[i]
        let mut activations = Vec::with_capacity(self.layers.len()+1);

        // Forward propagation
        
        let mut index = 0;
        activations.push(inputs.clone());
        for layer in &self.layers {
            let shape = layer.param_shape();

            let slice = unsafe {
                MatrixSlice::from_raw_parts(weights.as_ptr().offset(index as isize),
                                            shape.0,
                                            shape.1,
                                            shape.1)
            };

            let output = layer.forward(activations.last().unwrap(), slice);
            activations.push(output);
            index += layer.num_params();
        }
        let output = activations.last().unwrap();

        // Backward propagation

        //The gradient with respect to the current layer's output
        let mut out_grad = self.criterion.cost_grad(output, targets);
        // at this point index == weights.len()
        for (i, layer) in self.layers.iter().enumerate().rev() {
            index -= layer.num_params();
            let shape = layer.param_shape();

            let slice = unsafe {
                MatrixSlice::from_raw_parts(weights.as_ptr().offset(index as isize),
                                            shape.0,
                                            shape.1,
                                            shape.1)
            };

            let grad_params = layer.back_params(&out_grad, &activations[i], slice);
            out_grad = layer.back_input(&out_grad, &activations[i], slice);

            gradients[index..index+layer.num_params()].copy_from_slice(&grad_params.data());
        }

        let cost = self.criterion.cost(output, targets);
        (cost, gradients)
    }

    /// Forward propagation of the model weights to get the outputs.
    fn forward_prop(&self, inputs: &Matrix<f64>) -> Matrix<f64> {
        let mut index = 0;
        if self.layers.len() == 0 {
            return inputs.clone();
        }

        let mut outputs = unsafe {
            let shape = self.layers[0].param_shape();
            let slice = MatrixSlice::from_raw_parts(self.weights.as_ptr(),
                                                    shape.0,
                                                    shape.1,
                                                    shape.1);
            self.layers[0].forward(inputs, slice)
        };
        for layer in self.layers.iter().skip(1) {
            let shape = layer.param_shape();

            let slice = unsafe {
                MatrixSlice::from_raw_parts(self.weights.as_ptr().offset(index as isize),
                                            shape.0,
                                            shape.1,
                                            shape.1)
            };

            outputs = layer.forward(&outputs, slice);
            index += layer.num_params();
        }
        outputs
    }
}

/// Compute the gradient of the Neural Network using the
/// back propagation algorithm.
impl<T: Criterion> Optimizable for BaseNeuralNet<T> {
    type Inputs = Matrix<f64>;
    type Targets = Matrix<f64>;

    /// Compute the gradient of the neural network.
    fn compute_grad(&self,
                    params: &[f64],
                    inputs: &Matrix<f64>,
                    targets: &Matrix<f64>)
                    -> (f64, Vec<f64>) {
        self.compute_grad(params, inputs, targets)
    }
}

/// Criterion for Neural Networks
///
/// Specifies an activation function and a cost function.
pub trait Criterion {
    /// The activation function for the criterion.
    type ActFunc: ActivationFunc + Debug;
    /// The cost function for the criterion.
    type Cost: CostFunc<Matrix<f64>>;

    /// The activation function applied to a matrix.
    fn activate(&self, mat: Matrix<f64>) -> Matrix<f64> {
        mat.apply(&Self::ActFunc::func)
    }

    /// The gradient of the activation function applied to a matrix.
    fn grad_activ(&self, mat: Matrix<f64>) -> Matrix<f64> {
        mat.apply(&Self::ActFunc::func_grad)
    }

    /// The cost function.
    ///
    /// Returns a scalar cost.
    fn cost(&self, outputs: &Matrix<f64>, targets: &Matrix<f64>) -> f64 {
        Self::Cost::cost(outputs, targets)
    }

    /// The gradient of the cost function.
    ///
    /// Returns a matrix of cost gradients.
    fn cost_grad(&self, outputs: &Matrix<f64>, targets: &Matrix<f64>) -> Matrix<f64> {
        Self::Cost::grad_cost(outputs, targets)
    }

    /// Returns the regularization for this criterion.
    ///
    /// Will return `Regularization::None` by default.
    fn regularization(&self) -> Regularization<f64> {
        Regularization::None
    }

    /// Checks if the current criterion includes regularization.
    ///
    /// Will return `false` by default.
    fn is_regularized(&self) -> bool {
        match self.regularization() {
            Regularization::None => false,
            _ => true,
        }
    }

    /// Returns the regularization cost for the criterion.
    ///
    /// Will return `0` by default.
    ///
    /// This method will not be invoked by the neural network
    /// if there is explicitly no regularization.
    fn reg_cost(&self, reg_weights: MatrixSlice<f64>) -> f64 {
        self.regularization().reg_cost(reg_weights)
    }

    /// Returns the regularization gradient for the criterion.
    ///
    /// Will return a matrix of zeros by default.
    ///
    /// This method will not be invoked by the neural network
    /// if there is explicitly no regularization.
    fn reg_cost_grad(&self, reg_weights: MatrixSlice<f64>) -> Matrix<f64> {
        self.regularization().reg_grad(reg_weights)
    }
}

/// The binary cross entropy criterion.
///
/// Uses the Sigmoid activation function and the
/// cross entropy error.
#[derive(Clone, Copy, Debug)]
pub struct BCECriterion {
    regularization: Regularization<f64>,
}

impl Criterion for BCECriterion {
    type ActFunc = activ_fn::Sigmoid;
    type Cost = cost_fn::CrossEntropyError;

    fn regularization(&self) -> Regularization<f64> {
        self.regularization
    }
}

/// Creates an MSE Criterion without any regularization.
impl Default for BCECriterion {
    fn default() -> Self {
        BCECriterion { regularization: Regularization::None }
    }
}

impl BCECriterion {
    /// Constructs a new BCECriterion with the given regularization.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::learning::nnet::BCECriterion;
    /// use rusty_machine::learning::toolkit::regularization::Regularization;
    ///
    /// // Create a new BCE criterion with L2 regularization of 0.3.
    /// let criterion = BCECriterion::new(Regularization::L2(0.3f64));
    /// ```
    pub fn new(regularization: Regularization<f64>) -> Self {
        BCECriterion { regularization: regularization }
    }
}

/// The mean squared error criterion.
///
/// Uses the Linear activation function and the
/// mean squared error.
#[derive(Clone, Copy, Debug)]
pub struct MSECriterion {
    regularization: Regularization<f64>,
}

impl Criterion for MSECriterion {
    type ActFunc = activ_fn::Linear;
    type Cost = cost_fn::MeanSqError;

    fn regularization(&self) -> Regularization<f64> {
        self.regularization
    }
}

/// Creates an MSE Criterion without any regularization.
impl Default for MSECriterion {
    fn default() -> Self {
        MSECriterion { regularization: Regularization::None }
    }
}

impl MSECriterion {
    /// Constructs a new BCECriterion with the given regularization.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_machine::learning::nnet::MSECriterion;
    /// use rusty_machine::learning::toolkit::regularization::Regularization;
    ///
    /// // Create a new MSE criterion with L2 regularization of 0.3.
    /// let criterion = MSECriterion::new(Regularization::L2(0.3f64));
    /// ```
    pub fn new(regularization: Regularization<f64>) -> Self {
        MSECriterion { regularization: regularization }
    }
}

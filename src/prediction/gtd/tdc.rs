use core::*;
use domains::Transition;
use fa::{Approximator, Parameterised, Projection, Projector, SimpleLFA, VFunction};

pub struct TDC<S, P: Projector<S>> {
    pub fa_theta: Shared<SimpleLFA<S, P>>,
    pub fa_w: Shared<SimpleLFA<S, P>>,

    pub alpha: Parameter,
    pub beta: Parameter,
    pub gamma: Parameter,
}

impl<S, P: Projector<S>> TDC<S, P> {
    pub fn new<T1, T2, T3>(
        fa_theta: Shared<SimpleLFA<S, P>>,
        fa_w: Shared<SimpleLFA<S, P>>,
        alpha: T1,
        beta: T2,
        gamma: T3,
    ) -> Self
    where
        T1: Into<Parameter>,
        T2: Into<Parameter>,
        T3: Into<Parameter>,
    {
        if fa_theta.borrow().projector.dim() != fa_w.borrow().projector.dim() {
            panic!("fa_theta and fa_w must be equivalent function approximators.")
        }

        TDC {
            fa_theta,
            fa_w,

            alpha: alpha.into(),
            beta: beta.into(),
            gamma: gamma.into(),
        }
    }
}

impl<S, M: Projector<S>> Algorithm for TDC<S, M> {
    fn handle_terminal(&mut self) {
        self.alpha = self.alpha.step();
        self.beta = self.alpha.step();
        self.gamma = self.gamma.step();
    }
}

impl<S, A, M: Projector<S>> OnlineLearner<S, A> for TDC<S, M> {
    fn handle_transition(&mut self, t: &Transition<S, A>) {
        let phi_s = self.fa_theta.borrow().projector.project(&t.from.state());
        let phi_ns = self.fa_theta.borrow().projector.project(&t.to.state());

        let v = self.fa_theta.borrow().evaluate_phi(&phi_s);

        let td_estimate = self.fa_w.borrow().evaluate_phi(&phi_s);
        let td_error = if t.terminated() {
            t.reward - v
        } else {
            t.reward + self.gamma * self.fa_theta.borrow().evaluate_phi(&phi_ns) - v
        };

        self.fa_w.borrow_mut().update_phi(&phi_s, self.beta * (td_error - td_estimate));

        let dim = self.fa_theta.borrow().projector.dim();
        let phi =
            td_error * phi_s.expanded(dim) -
            td_estimate * self.gamma.value() * phi_ns.expanded(dim);

        self.fa_theta.borrow_mut().update_phi(&Projection::Dense(phi), self.alpha.value());
    }
}

impl<S, P: Projector<S>> ValuePredictor<S> for TDC<S, P> {
    fn predict_v(&mut self, s: &S) -> f64 {
        self.fa_theta.borrow().evaluate(s).unwrap()
    }
}

impl<S, A, P: Projector<S>> ActionValuePredictor<S, A> for TDC<S, P> {}

impl<S, P: Projector<S>> Parameterised for TDC<S, P> {
    fn weights(&self) -> Matrix<f64> {
        self.fa_theta.borrow().weights()
    }
}

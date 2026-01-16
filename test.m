function x_est = FlightEstimator(x_est,constantsASTRA,z,dT,P0)
%% M-EKF Implementation
% Remove bias from IMU
z(1:3) = z(1:3) - x_est(14:16);
z(4:6) = z(4:6) - x_est(11:13);
z(7:9) = z(7:9) - x_est(17:19);

% Extract quaternion
dx = zeros(9,1);
q = x_est(1:4);
qdot = 0.5 * HamiltonianProd(q) * [0; z(4:6)]; 
x_est(1:4) = q + qdot * dT;
q = x_est(1:4);

% A-priori quaternion estimate and rotation matrix
q = q / norm(q);
R_b2i = quatRot(q)';

% Process Covariance Matrix
persistent P lastZ 
if isempty(P)
    P = P0;  
    lastZ = zeros(15,1);
end

% State Transition Matrix
F = StateTransitionMat(z(1:3), z(4:6), R_b2i, 0);

% Propagate rest of state using IMU
x_est(8:10) = x_est(8:10) + (R_b2i * z(1:3) - [0; 0; constantsASTRA.g]) * dT;
x_est(5:7) = x_est(5:7) + x_est(8:10) * dT;

% Discrete STM
Phi = expm(F * dT);

% Extract Matrices
Q = constantsASTRA.Q(1:9,1:9);

% Process Noise Covariance and a-priori propagation step
Q = 0.5 * Q;
P = Phi * P * Phi' + Q;
RTK = 1;

end
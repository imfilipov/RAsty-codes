root = fileparts(fileparts(fileparts(mfilename('fullpath'))));
addpath(fullfile(root, 'tests', 'octave_reference'));
function H = read_edges(root, filename)
  path = fullfile(root, 'data', filename);
  fid = fopen(path, 'r');
  dims = fscanf(fid, '%d', 2);
  fclose(fid);
  edges = dlmread(path, ' ', 1, 0);
  H = sparse(edges(:,1)+1, edges(:,2)+1, 1, dims(1), dims(2));
endfunction

cases = { 'RA_n6000_R150.edges', 'alice_ra6k.txt', 'message_ra6k.txt', 0.01, 'RA6K' };
if strcmp(getenv('RA_TEST_DVB'), '1')
  cases(end+1,:) = { 'DVB_S2_110.edges', 'alice_dvb.txt', 'message_dvb.txt', 0.08, 'DVB' };
endif
for c = 1:rows(cases)
  H = read_edges(root, cases{c,1});
  A = char(strtrim(fileread(fullfile(root, 'data', cases{c,2}))));
  M = char(strtrim(fileread(fullfile(root, 'data', cases{c,3}))));
  tic();
  output = evalc(sprintf('[estimate, success] = belief_propagation_DV(A, M, H, %.17g, 100);', cases{c,4}));
  elapsed_ms = toc()*1000;
  estimate = char(estimate);
  fprintf('%s estimate_length=%d success=%d estimate_ones=%d elapsed_ms=%.3f\n', ...
          cases{c,5}, length(estimate), success, sum(estimate == '1'), elapsed_ms);
  fid = fopen(fullfile(tempdir(), ['ra-rust-', cases{c,5}, '-octave-estimate.txt']), 'w');
  fprintf(fid, '%s\n', estimate);
  fclose(fid);
  assert(success == 1);
  assert(length(estimate) == columns(H));
endfor
